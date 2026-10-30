use chrono::{NaiveDate, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::models::conference::*;

#[derive(Clone)]
pub struct ConferenceDao {
    pool: Pool,
}

impl ConferenceDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn compute_status(conference_date: NaiveDate) -> String {
        if conference_date >= Utc::now().date_naive() {
            "active".to_string()
        } else {
            "expired".to_string()
        }
    }

    fn format_time(t: chrono::NaiveTime) -> String {
        t.format("%H:%M").to_string()
    }

    // ── Create ────────────────────────────────────────────────────────────────

    pub async fn create_conference(
        &self,
        req: &CreateConferenceRequest,
        created_by: Uuid,
    ) -> ApiResult<ConferenceListItem> {
        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let tx = client.transaction().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = tx.query_one(
            r#"INSERT INTO conferences
               (school_id, title, description, conference_date, image_url, booking_close_at, created_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, created_at"#,
            &[
                &req.school_id, &req.title, &req.description, &req.conference_date,
                &req.image_url, &req.booking_close_at, &created_by,
            ],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let conf_id: Uuid = row.get("id");
        let created_at = row.get("created_at");

        for cr in &req.classrooms {
            tx.execute(
                "INSERT INTO conference_classrooms (conference_id, classroom_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&conf_id, &cr.classroom_id],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;

            for s in &cr.slots {
                let start: chrono::NaiveTime = s.start_time.parse()
                    .map_err(|_| AppError::Validation(format!("Invalid start_time: {}", s.start_time)))?;
                let end: chrono::NaiveTime = s.end_time.parse()
                    .map_err(|_| AppError::Validation(format!("Invalid end_time: {}", s.end_time)))?;
                tx.execute(
                    "INSERT INTO conference_slots (conference_id, classroom_id, start_time, end_time, capacity) VALUES ($1, $2, $3, $4, $5)",
                    &[&conf_id, &cr.classroom_id, &start, &end, &s.capacity],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;

        self.get_conference_list_item(conf_id, created_at).await
    }

    async fn get_conference_list_item(
        &self,
        conf_id: Uuid,
        created_at: chrono::DateTime<Utc>,
    ) -> ApiResult<ConferenceListItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = client.query_one(
            r#"SELECT c.id, c.school_id, c.title, c.description, c.conference_date,
                      c.image_url, c.booking_close_at,
                      COALESCE(SUM(cs.capacity), 0)::BIGINT AS total_slots,
                      COALESCE(SUM(CASE WHEN cb.status = 'booked' THEN 1 ELSE 0 END), 0)::BIGINT AS booked_slots
               FROM conferences c
               LEFT JOIN conference_slots cs ON cs.conference_id = c.id
               LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id AND cb.status = 'booked'
               WHERE c.id = $1
               GROUP BY c.id, c.school_id, c.title, c.description, c.conference_date, c.image_url, c.booking_close_at"#,
            &[&conf_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let total_slots: i64 = row.get("total_slots");
        let booked_slots: i64 = row.get("booked_slots");
        let conference_date: NaiveDate = row.get("conference_date");

        let classroom_tags = self.get_classroom_tags(conf_id).await?;

        Ok(ConferenceListItem {
            id: row.get("id"),
            school_id: row.get("school_id"),
            title: row.get("title"),
            description: row.get("description"),
            conference_date,
            image_url: row.get("image_url"),
            booking_close_at: row.get("booking_close_at"),
            status: Self::compute_status(conference_date),
            classroom_tags,
            total_slots,
            booked_slots,
            available_slots: (total_slots - booked_slots).max(0),
            created_at,
        })
    }

    async fn get_classroom_tags(&self, conf_id: Uuid) -> ApiResult<Vec<ClassroomTag>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = client.query(
            r#"SELECT cc.classroom_id, cl.name AS classroom_name
               FROM conference_classrooms cc
               JOIN classrooms cl ON cl.id = cc.classroom_id
               WHERE cc.conference_id = $1
               ORDER BY cl.name"#,
            &[&conf_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| ClassroomTag {
            classroom_id: r.get("classroom_id"),
            classroom_name: r.get("classroom_name"),
        }).collect())
    }

    // ── List (admin) ──────────────────────────────────────────────────────────

    pub async fn list_conferences(&self, school_id: Uuid) -> ApiResult<Vec<ConferenceListItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"SELECT c.id, c.school_id, c.title, c.description, c.conference_date,
                      c.image_url, c.booking_close_at, c.created_at,
                      COALESCE(SUM(cs.capacity), 0)::BIGINT AS total_slots,
                      COALESCE(SUM(CASE WHEN cb.status = 'booked' THEN 1 ELSE 0 END), 0)::BIGINT AS booked_slots
               FROM conferences c
               LEFT JOIN conference_slots cs ON cs.conference_id = c.id
               LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id AND cb.status = 'booked'
               WHERE c.school_id = $1
               GROUP BY c.id, c.school_id, c.title, c.description, c.conference_date,
                        c.image_url, c.booking_close_at, c.created_at
               ORDER BY c.conference_date DESC, c.created_at DESC"#,
            &[&school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let mut items = Vec::new();
        for row in &rows {
            let conf_id: Uuid = row.get("id");
            let total_slots: i64 = row.get("total_slots");
            let booked_slots: i64 = row.get("booked_slots");
            let conference_date: NaiveDate = row.get("conference_date");
            let classroom_tags = self.get_classroom_tags(conf_id).await?;

            items.push(ConferenceListItem {
                id: conf_id,
                school_id: row.get("school_id"),
                title: row.get("title"),
                description: row.get("description"),
                conference_date,
                image_url: row.get("image_url"),
                booking_close_at: row.get("booking_close_at"),
                status: Self::compute_status(conference_date),
                classroom_tags,
                total_slots,
                booked_slots,
                available_slots: (total_slots - booked_slots).max(0),
                created_at: row.get("created_at"),
            });
        }
        Ok(items)
    }

    // ── Detail ────────────────────────────────────────────────────────────────

    pub async fn get_conference_detail(&self, id: Uuid) -> ApiResult<Option<ConferenceDetail>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row_opt = client.query_opt(
            r#"SELECT c.id, c.school_id, c.title, c.description, c.conference_date,
                      c.image_url, c.booking_close_at, c.created_at,
                      COALESCE(SUM(cs.capacity), 0)::BIGINT AS total_slots,
                      COALESCE(SUM(CASE WHEN cb.status = 'booked' THEN 1 ELSE 0 END), 0)::BIGINT AS booked_slots
               FROM conferences c
               LEFT JOIN conference_slots cs ON cs.conference_id = c.id
               LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id AND cb.status = 'booked'
               WHERE c.id = $1
               GROUP BY c.id, c.school_id, c.title, c.description, c.conference_date,
                        c.image_url, c.booking_close_at, c.created_at"#,
            &[&id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let conf_id: Uuid = row.get("id");
        let total_slots: i64 = row.get("total_slots");
        let booked_slots: i64 = row.get("booked_slots");
        let conference_date: NaiveDate = row.get("conference_date");
        let classroom_tags = self.get_classroom_tags(conf_id).await?;

        let conference = ConferenceListItem {
            id: conf_id,
            school_id: row.get("school_id"),
            title: row.get("title"),
            description: row.get("description"),
            conference_date,
            image_url: row.get("image_url"),
            booking_close_at: row.get("booking_close_at"),
            status: Self::compute_status(conference_date),
            classroom_tags: classroom_tags.clone(),
            total_slots,
            booked_slots,
            available_slots: (total_slots - booked_slots).max(0),
            created_at: row.get("created_at"),
        };

        let mut classrooms_detail = Vec::new();
        for tag in &classroom_tags {
            let slots = self.get_slots_for_classroom_internal(conf_id, tag.classroom_id, None).await?;
            classrooms_detail.push(ClassroomSlotsResponse {
                classroom_id: tag.classroom_id,
                classroom_name: tag.classroom_name.clone(),
                slots,
            });
        }

        Ok(Some(ConferenceDetail { conference, classrooms_detail }))
    }

    // ── Update ────────────────────────────────────────────────────────────────

    pub async fn update_conference(&self, req: &UpdateConferenceRequest) -> ApiResult<ConferenceListItem> {
        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let tx = client.transaction().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = tx.query_one(
            r#"UPDATE conferences SET
               title            = COALESCE($2, title),
               description      = COALESCE($3, description),
               conference_date  = COALESCE($4, conference_date),
               image_url        = CASE WHEN $5::TEXT IS NOT NULL THEN $5 ELSE image_url END,
               booking_close_at = CASE WHEN $6::TIMESTAMPTZ IS NOT NULL THEN $6 ELSE booking_close_at END,
               updated_at       = NOW()
               WHERE id = $1
               RETURNING id, created_at"#,
            &[&req.id, &req.title, &req.description, &req.conference_date,
              &req.image_url, &req.booking_close_at],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let conf_id: Uuid = row.get("id");
        let created_at = row.get("created_at");

        if let Some(classrooms) = &req.classrooms {
            tx.execute("DELETE FROM conference_classrooms WHERE conference_id = $1", &[&conf_id])
                .await.map_err(|e| AppError::Database(e.to_string()))?;

            for cr in classrooms {
                tx.execute(
                    "INSERT INTO conference_classrooms (conference_id, classroom_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    &[&conf_id, &cr.classroom_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;

                // Fetch existing slots with booking counts for this classroom
                let existing = tx.query(
                    r#"SELECT cs.id,
                              COALESCE(COUNT(cb.id), 0)::BIGINT AS booking_count
                       FROM conference_slots cs
                       LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id
                       WHERE cs.conference_id = $1 AND cs.classroom_id = $2
                       GROUP BY cs.id"#,
                    &[&conf_id, &cr.classroom_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;

                use std::collections::{HashMap, HashSet};
                let existing_counts: HashMap<Uuid, i64> = existing.iter()
                    .map(|r| (r.get::<_, Uuid>("id"), r.get::<_, i64>("booking_count")))
                    .collect();

                let payload_ids: HashSet<Uuid> = cr.slots.iter()
                    .filter_map(|s| s.id)
                    .collect();

                // Delete removed slots — only if they have no bookings
                for (slot_id, booking_count) in &existing_counts {
                    if !payload_ids.contains(slot_id) {
                        if *booking_count > 0 {
                            return Err(AppError::Validation(
                                "Cannot remove a slot that has existing bookings".to_string()
                            ));
                        }
                        tx.execute("DELETE FROM conference_slots WHERE id = $1", &[slot_id])
                            .await.map_err(|e| AppError::Database(e.to_string()))?;
                    }
                }

                // Update existing slots / insert new ones
                for s in &cr.slots {
                    let start: chrono::NaiveTime = s.start_time.parse()
                        .map_err(|_| AppError::Validation(format!("Invalid start_time: {}", s.start_time)))?;
                    let end: chrono::NaiveTime = s.end_time.parse()
                        .map_err(|_| AppError::Validation(format!("Invalid end_time: {}", s.end_time)))?;

                    if let Some(slot_id) = s.id {
                        tx.execute(
                            "UPDATE conference_slots SET start_time = $2, end_time = $3, capacity = $4 WHERE id = $1",
                            &[&slot_id, &start, &end, &s.capacity],
                        ).await.map_err(|e| AppError::Database(e.to_string()))?;
                    } else {
                        tx.execute(
                            "INSERT INTO conference_slots (conference_id, classroom_id, start_time, end_time, capacity) VALUES ($1, $2, $3, $4, $5)",
                            &[&conf_id, &cr.classroom_id, &start, &end, &s.capacity],
                        ).await.map_err(|e| AppError::Database(e.to_string()))?;
                    }
                }
            }
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;

        self.get_conference_list_item(conf_id, created_at).await
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    pub async fn delete_conference(&self, id: Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        client.execute("DELETE FROM conferences WHERE id = $1", &[&id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_conference_booking_count(&self, conference_id: Uuid) -> ApiResult<i64> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "SELECT COUNT(*)::BIGINT AS count FROM conference_bookings \
             WHERE conference_id = $1 AND status = 'booked'",
            &[&conference_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.get("count"))
    }

    // ── Stats ─────────────────────────────────────────────────────────────────

    pub async fn get_conference_stats(&self, school_id: Uuid) -> ApiResult<ConferenceStats> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            r#"SELECT
               COUNT(DISTINCT c.id)::BIGINT AS total_conferences,
               COALESCE(SUM(cs.capacity), 0)::BIGINT AS total_slots,
               COALESCE((SELECT COUNT(*) FROM conference_bookings cb2
                         JOIN conferences c2 ON c2.id = cb2.conference_id
                         WHERE c2.school_id = $1 AND cb2.status = 'booked'), 0)::BIGINT AS booked_slots
               FROM conferences c
               LEFT JOIN conference_slots cs ON cs.conference_id = c.id
               WHERE c.school_id = $1"#,
            &[&school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let total_slots: i64 = row.get("total_slots");
        let booked_slots: i64 = row.get("booked_slots");

        Ok(ConferenceStats {
            total_conferences: row.get("total_conferences"),
            total_slots,
            booked_slots,
            available_slots: (total_slots - booked_slots).max(0),
        })
    }

    // ── Slots ─────────────────────────────────────────────────────────────────

    async fn get_slots_for_classroom_internal(
        &self,
        conference_id: Uuid,
        classroom_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> ApiResult<Vec<SlotDetail>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"SELECT cs.id, cs.start_time, cs.end_time, cs.capacity,
                      COUNT(CASE WHEN cb.status = 'booked' THEN 1 END)::BIGINT AS booked_count,
                      (SELECT cb2.id FROM conference_bookings cb2
                       WHERE cb2.slot_id = cs.id AND cb2.parent_id = $3 AND cb2.status = 'booked'
                       LIMIT 1) AS my_booking_id
               FROM conference_slots cs
               LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id AND cb.status = 'booked'
               WHERE cs.conference_id = $1 AND cs.classroom_id = $2
               GROUP BY cs.id, cs.start_time, cs.end_time, cs.capacity
               ORDER BY cs.start_time"#,
            &[&conference_id, &classroom_id, &parent_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| {
            let capacity: i32 = r.get("capacity");
            let booked_count: i64 = r.get("booked_count");
            SlotDetail {
                id: r.get("id"),
                start_time: Self::format_time(r.get("start_time")),
                end_time: Self::format_time(r.get("end_time")),
                capacity,
                booked_count,
                is_available: booked_count < capacity as i64,
                my_booking_id: r.get("my_booking_id"),
            }
        }).collect())
    }

    pub async fn get_slots_for_classroom(
        &self,
        conference_id: Uuid,
        classroom_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> ApiResult<Vec<SlotDetail>> {
        self.get_slots_for_classroom_internal(conference_id, classroom_id, parent_id).await
    }

    // ── Booking ───────────────────────────────────────────────────────────────

    pub async fn check_slot_capacity(&self, slot_id: Uuid) -> ApiResult<(i32, i64)> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            r#"SELECT cs.capacity,
                      COUNT(CASE WHEN cb.status = 'booked' THEN 1 END)::BIGINT AS booked_count
               FROM conference_slots cs
               LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id
               WHERE cs.id = $1
               GROUP BY cs.capacity"#,
            &[&slot_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok((row.get("capacity"), row.get("booked_count")))
    }

    pub async fn create_booking(
        &self,
        req: &BookSlotRequest,
        parent_id: Uuid,
    ) -> ApiResult<BookingListItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let slot_row = client.query_one(
            "SELECT classroom_id FROM conference_slots WHERE id = $1",
            &[&req.slot_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        let classroom_id: Uuid = slot_row.get("classroom_id");

        let row = client.query_one(
            r#"INSERT INTO conference_bookings
               (conference_id, slot_id, parent_id, child_id, classroom_id, school_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (conference_id, classroom_id, child_id)
               DO UPDATE SET slot_id = EXCLUDED.slot_id,
                             classroom_id = EXCLUDED.classroom_id,
                             status = 'booked',
                             updated_at = NOW()
               RETURNING id, booked_at"#,
            &[&req.conference_id, &req.slot_id, &parent_id, &req.child_id, &classroom_id, &req.school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let booking_id: Uuid = row.get("id");
        self.get_booking_detail(booking_id).await?
            .ok_or_else(|| AppError::Internal("Booking not found after insert".to_string()))
    }

    pub async fn get_booking_detail(&self, booking_id: Uuid) -> ApiResult<Option<BookingListItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row_opt = client.query_opt(
            r#"SELECT cb.id, cb.conference_id, c.title AS conference_title, c.conference_date,
                      c.image_url,
                      cb.slot_id, cs.start_time, cs.end_time,
                      cb.child_id, ch.first_name || ' ' || ch.last_name AS child_name,
                      cb.classroom_id, cl.name AS classroom_name,
                      cb.booked_at, cb.status
               FROM conference_bookings cb
               JOIN conferences c ON c.id = cb.conference_id
               JOIN conference_slots cs ON cs.id = cb.slot_id
               JOIN children ch ON ch.id = cb.child_id
               JOIN classrooms cl ON cl.id = cb.classroom_id
               WHERE cb.id = $1"#,
            &[&booking_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row_opt.map(|r| BookingListItem {
            id: r.get("id"),
            conference_id: r.get("conference_id"),
            conference_title: r.get("conference_title"),
            conference_date: r.get("conference_date"),
            image_url: r.get("image_url"),
            slot_id: r.get("slot_id"),
            slot_start: Self::format_time(r.get("start_time")),
            slot_end: Self::format_time(r.get("end_time")),
            child_id: r.get("child_id"),
            child_name: r.get("child_name"),
            classroom_id: r.get("classroom_id"),
            classroom_name: r.get("classroom_name"),
            booked_at: r.get("booked_at"),
            status: r.get("status"),
        }))
    }

    pub async fn list_my_bookings(
        &self,
        parent_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<Vec<BookingListItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"SELECT cb.id, cb.conference_id, c.title AS conference_title, c.conference_date,
                      c.image_url,
                      cb.slot_id, cs.start_time, cs.end_time,
                      cb.child_id, ch.first_name || ' ' || ch.last_name AS child_name,
                      cb.classroom_id, cl.name AS classroom_name,
                      cb.booked_at, cb.status
               FROM conference_bookings cb
               JOIN conferences c ON c.id = cb.conference_id
               JOIN conference_slots cs ON cs.id = cb.slot_id
               JOIN children ch ON ch.id = cb.child_id
               JOIN classrooms cl ON cl.id = cb.classroom_id
               WHERE cb.parent_id = $1 AND cb.school_id = $2 AND cb.status = 'booked'
               ORDER BY c.conference_date ASC, cs.start_time ASC"#,
            &[&parent_id, &school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| BookingListItem {
            id: r.get("id"),
            conference_id: r.get("conference_id"),
            conference_title: r.get("conference_title"),
            conference_date: r.get("conference_date"),
            image_url: r.get("image_url"),
            slot_id: r.get("slot_id"),
            slot_start: Self::format_time(r.get("start_time")),
            slot_end: Self::format_time(r.get("end_time")),
            child_id: r.get("child_id"),
            child_name: r.get("child_name"),
            classroom_id: r.get("classroom_id"),
            classroom_name: r.get("classroom_name"),
            booked_at: r.get("booked_at"),
            status: r.get("status"),
        }).collect())
    }

    pub async fn update_booking(
        &self,
        booking_id: Uuid,
        new_slot_id: Uuid,
        parent_id: Uuid,
    ) -> ApiResult<BookingListItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let slot_row = client.query_one(
            "SELECT classroom_id FROM conference_slots WHERE id = $1",
            &[&new_slot_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        let classroom_id: Uuid = slot_row.get("classroom_id");

        client.execute(
            r#"UPDATE conference_bookings
               SET slot_id = $2, classroom_id = $3, updated_at = NOW()
               WHERE id = $1 AND parent_id = $4 AND status = 'booked'"#,
            &[&booking_id, &new_slot_id, &classroom_id, &parent_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        self.get_booking_detail(booking_id).await?
            .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))
    }

    pub async fn cancel_booking(&self, booking_id: Uuid, parent_id: Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        client.execute(
            "UPDATE conference_bookings SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND parent_id = $2",
            &[&booking_id, &parent_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ── Parent-visible conferences ─────────────────────────────────────────────

    pub async fn list_available_for_parent(
        &self,
        parent_id: Uuid,
        school_id: Uuid,
        child_id: Option<Uuid>,
    ) -> ApiResult<Vec<ConferenceListItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Conferences that have at least one classroom containing parent's child(ren)
        let rows = client.query(
            r#"SELECT DISTINCT c.id, c.school_id, c.title, c.description, c.conference_date,
                      c.image_url, c.booking_close_at, c.created_at
               FROM conferences c
               JOIN conference_classrooms cc ON cc.conference_id = c.id
               JOIN enrollments e ON e.classroom_id = cc.classroom_id
               JOIN children ch ON ch.id = e.child_id
               WHERE c.school_id = $1
                 AND (ch.parent_id = $2 OR ch.secondary_parent_id = $2)
                 AND ch.status = 'active'
                 AND ($3::UUID IS NULL OR ch.id = $3)
               ORDER BY c.conference_date ASC"#,
            &[&school_id, &parent_id, &child_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let mut items = Vec::new();
        for row in &rows {
            let conf_id: Uuid = row.get("id");

            let stat_row = client.query_one(
                r#"SELECT COALESCE(SUM(cs.capacity), 0)::BIGINT AS total_slots,
                          COALESCE(SUM(CASE WHEN cb.status = 'booked' THEN 1 ELSE 0 END), 0)::BIGINT AS booked_slots
                   FROM conference_slots cs
                   LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id AND cb.status = 'booked'
                   WHERE cs.conference_id = $1"#,
                &[&conf_id],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;

            let total_slots: i64 = stat_row.get("total_slots");
            let booked_slots: i64 = stat_row.get("booked_slots");
            let conference_date: NaiveDate = row.get("conference_date");
            let classroom_tags = self.get_classroom_tags(conf_id).await?;

            items.push(ConferenceListItem {
                id: conf_id,
                school_id: row.get("school_id"),
                title: row.get("title"),
                description: row.get("description"),
                conference_date,
                image_url: row.get("image_url"),
                booking_close_at: row.get("booking_close_at"),
                status: Self::compute_status(conference_date),
                classroom_tags,
                total_slots,
                booked_slots,
                available_slots: (total_slots - booked_slots).max(0),
                created_at: row.get("created_at"),
            });
        }
        Ok(items)
    }

    // ── Report ────────────────────────────────────────────────────────────────

    pub async fn get_conference_report(&self, id: Uuid) -> ApiResult<Option<ConferenceReportResponse>> {
        let conference = match self.get_conference_list_item_by_id(id).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let classrooms = self.get_classroom_tags(id).await?;
        let mut report = Vec::new();

        for tag in &classrooms {
            // Count enrolled children
            let count_row = client.query_one(
                r#"SELECT COUNT(DISTINCT e.child_id)::BIGINT AS student_count
                   FROM enrollments e
                   JOIN children ch ON ch.id = e.child_id
                   WHERE e.classroom_id = $1 AND ch.status = 'active'"#,
                &[&tag.classroom_id],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;
            let student_count: i64 = count_row.get("student_count");

            // Slots with bookings
            let slot_rows = client.query(
                r#"SELECT cs.id, cs.start_time, cs.end_time, cs.capacity,
                          COUNT(CASE WHEN cb.status = 'booked' THEN 1 END)::BIGINT AS booked_count
                   FROM conference_slots cs
                   LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id
                   WHERE cs.conference_id = $1 AND cs.classroom_id = $2
                   GROUP BY cs.id, cs.start_time, cs.end_time, cs.capacity
                   ORDER BY cs.start_time"#,
                &[&id, &tag.classroom_id],
            ).await.map_err(|e| AppError::Database(e.to_string()))?;

            let mut slots = Vec::new();
            for sr in &slot_rows {
                let slot_id: Uuid = sr.get("id");
                let capacity: i32 = sr.get("capacity");
                let booked_count: i64 = sr.get("booked_count");

                let booking_rows = client.query(
                    r#"SELECT cb.id AS booking_id,
                              u.first_name || ' ' || u.last_name AS parent_name,
                              u.email AS parent_email,
                              cb.child_id,
                              ch.first_name || ' ' || ch.last_name AS child_name,
                              cb.booked_at
                       FROM conference_bookings cb
                       JOIN users u ON u.id = cb.parent_id
                       JOIN children ch ON ch.id = cb.child_id
                       WHERE cb.slot_id = $1 AND cb.status = 'booked'"#,
                    &[&slot_id],
                ).await.map_err(|e| AppError::Database(e.to_string()))?;

                let bookings: Vec<BookingInfo> = booking_rows.iter().map(|br| BookingInfo {
                    booking_id: br.get("booking_id"),
                    parent_name: br.get("parent_name"),
                    parent_email: br.get("parent_email"),
                    child_id: br.get("child_id"),
                    child_name: br.get("child_name"),
                    booked_at: br.get("booked_at"),
                }).collect();

                slots.push(SlotReport {
                    slot_id,
                    start_time: Self::format_time(sr.get("start_time")),
                    end_time: Self::format_time(sr.get("end_time")),
                    capacity,
                    booked_count,
                    available: (capacity as i64 - booked_count).max(0),
                    bookings,
                });
            }

            // Unbooked parents
            let unbooked = self.get_unbooked_parents_for_classroom(id, tag.classroom_id).await?;
            let unbooked_count = unbooked.len() as i64;

            report.push(ClassroomReport {
                classroom_id: tag.classroom_id,
                classroom_name: tag.classroom_name.clone(),
                student_count,
                unbooked_count,
                slots,
                unbooked_parents: unbooked,
            });
        }

        Ok(Some(ConferenceReportResponse { conference, report }))
    }

    async fn get_conference_list_item_by_id(&self, id: Uuid) -> ApiResult<Option<ConferenceListItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row_opt = client.query_opt(
            r#"SELECT c.id, c.school_id, c.title, c.description, c.conference_date,
                      c.image_url, c.booking_close_at, c.created_at,
                      COALESCE(SUM(cs.capacity), 0)::BIGINT AS total_slots,
                      COALESCE(SUM(CASE WHEN cb.status = 'booked' THEN 1 ELSE 0 END), 0)::BIGINT AS booked_slots
               FROM conferences c
               LEFT JOIN conference_slots cs ON cs.conference_id = c.id
               LEFT JOIN conference_bookings cb ON cb.slot_id = cs.id AND cb.status = 'booked'
               WHERE c.id = $1
               GROUP BY c.id, c.school_id, c.title, c.description, c.conference_date,
                        c.image_url, c.booking_close_at, c.created_at"#,
            &[&id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row_opt.map(|row| {
            let total_slots: i64 = row.get("total_slots");
            let booked_slots: i64 = row.get("booked_slots");
            let conference_date: NaiveDate = row.get("conference_date");
            ConferenceListItem {
                id: row.get("id"),
                school_id: row.get("school_id"),
                title: row.get("title"),
                description: row.get("description"),
                conference_date,
                image_url: row.get("image_url"),
                booking_close_at: row.get("booking_close_at"),
                status: Self::compute_status(conference_date),
                classroom_tags: vec![],
                total_slots,
                booked_slots,
                available_slots: (total_slots - booked_slots).max(0),
                created_at: row.get("created_at"),
            }
        }))
    }

    pub async fn get_unbooked_parents_for_classroom(
        &self,
        conference_id: Uuid,
        classroom_id: Uuid,
    ) -> ApiResult<Vec<UnbookedParent>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"SELECT u.id AS parent_id,
                      u.first_name || ' ' || u.last_name AS parent_name,
                      u.email AS parent_email,
                      ch.id AS child_id,
                      ch.first_name || ' ' || ch.last_name AS child_name
               FROM enrollments e
               JOIN children ch ON ch.id = e.child_id
               JOIN users u ON (u.id = ch.parent_id OR u.id = ch.secondary_parent_id)
               WHERE e.classroom_id = $2
                 AND ch.status = 'active'
                 AND NOT EXISTS (
                   SELECT 1 FROM conference_bookings cb
                   WHERE cb.conference_id = $1
                     AND cb.child_id = ch.id
                     AND cb.classroom_id = $2
                     AND cb.status = 'booked'
                 )
               ORDER BY u.first_name, u.last_name"#,
            &[&conference_id, &classroom_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| UnbookedParent {
            parent_id: r.get("parent_id"),
            parent_name: r.get("parent_name"),
            parent_email: r.get("parent_email"),
            child_id: r.get("child_id"),
            child_name: r.get("child_name"),
        }).collect())
    }

    pub async fn list_all_unbooked_parents(&self, conference_id: Uuid) -> ApiResult<Vec<UnbookedParent>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"SELECT DISTINCT u.id AS parent_id,
                      COALESCE(u.first_name, '') || ' ' || COALESCE(u.last_name, '') AS parent_name,
                      COALESCE(u.email, '') AS parent_email,
                      ch.id AS child_id,
                      COALESCE(ch.first_name, '') || ' ' || COALESCE(ch.last_name, '') AS child_name
               FROM conference_classrooms cc
               JOIN enrollments e ON e.classroom_id = cc.classroom_id
               JOIN children ch ON ch.id = e.child_id
               JOIN users u ON (u.id = ch.parent_id OR u.id = ch.secondary_parent_id)
               WHERE cc.conference_id = $1
                 AND ch.status = 'active'
                 AND COALESCE(u.email, '') != ''
                 AND NOT EXISTS (
                   SELECT 1 FROM conference_bookings cb
                   WHERE cb.conference_id = $1
                     AND cb.child_id = ch.id
                     AND cb.status = 'booked'
                 )
               ORDER BY parent_name"#,
            &[&conference_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| UnbookedParent {
            parent_id: r.get("parent_id"),
            parent_name: r.get("parent_name"),
            parent_email: r.get("parent_email"),
            child_id: r.get("child_id"),
            child_name: r.get("child_name"),
        }).collect())
    }

    // ── Parent email list for conference created notification ──────────────────

    pub async fn list_parents_for_conference(&self, conference_id: Uuid) -> ApiResult<Vec<UnbookedParent>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"SELECT DISTINCT u.id AS parent_id,
                      COALESCE(u.first_name, '') || ' ' || COALESCE(u.last_name, '') AS parent_name,
                      COALESCE(u.email, '') AS parent_email,
                      ch.id AS child_id,
                      COALESCE(ch.first_name, '') || ' ' || COALESCE(ch.last_name, '') AS child_name
               FROM conference_classrooms cc
               JOIN enrollments e ON e.classroom_id = cc.classroom_id
               JOIN children ch ON ch.id = e.child_id
               JOIN users u ON (u.id = ch.parent_id OR u.id = ch.secondary_parent_id)
               WHERE cc.conference_id = $1
                 AND ch.status = 'active'
                 AND COALESCE(u.email, '') != ''
               ORDER BY parent_name"#,
            &[&conference_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| UnbookedParent {
            parent_id: r.get("parent_id"),
            parent_name: r.get("parent_name"),
            parent_email: r.get("parent_email"),
            child_id: r.get("child_id"),
            child_name: r.get("child_name"),
        }).collect())
    }

    // ── School name lookup (for emails) ──────────────────────────────────────

    pub async fn get_school_name(&self, school_id: Uuid) -> ApiResult<String> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "SELECT COALESCE(name, 'The Goddard School') AS name FROM schools WHERE id = $1",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.get("name"))
    }

    // ── Existing booking lookup ───────────────────────────────────────────────

    pub async fn find_existing_booking(
        &self,
        conference_id: Uuid,
        classroom_id: Uuid,
        child_id: Uuid,
    ) -> ApiResult<Option<Uuid>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "SELECT id FROM conference_bookings WHERE conference_id = $1 AND classroom_id = $2 AND child_id = $3 AND status = 'booked'",
            &[&conference_id, &classroom_id, &child_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.map(|r| r.get("id")))
    }

    pub async fn get_slot_classroom(&self, slot_id: Uuid) -> ApiResult<Uuid> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "SELECT classroom_id FROM conference_slots WHERE id = $1",
            &[&slot_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.get("classroom_id"))
    }
}
