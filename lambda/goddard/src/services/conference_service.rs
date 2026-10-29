use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

use crate::dao::conference_dao::ConferenceDao;
use crate::error::{ApiResult, AppError};
use crate::middleware::auth::AuthContext;
use crate::models::conference::*;
use crate::models::schema::UserRole;
use crate::services::email_service::EmailService;
use crate::services::email_templates;
use crate::services::upload_service::UploadService;

fn parent_dashboard_url() -> String {
    std::env::var("PARENT_DASHBOARD_URL")
        .unwrap_or_else(|_| "https://app.goddardschool.com".to_string())
}

#[derive(Clone)]
pub struct ConferenceService {
    dao: ConferenceDao,
    email: Arc<EmailService>,
    upload: Arc<UploadService>,
}

impl ConferenceService {
    pub fn new(dao: ConferenceDao, email: Arc<EmailService>, upload: Arc<UploadService>) -> Self {
        Self { dao, email, upload }
    }

    // ── Admin: create ─────────────────────────────────────────────────────────

    pub async fn create_conference(
        &self,
        mut req: CreateConferenceRequest,
        auth: &AuthContext,
    ) -> ApiResult<ConferenceListItem> {
        req.school_id = auth.school_id;

        // If image sent as base64, upload to S3 and set image_url
        if req.image_url.is_none() {
            if let Some(ref b64) = req.image_data {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|_| AppError::Validation("Invalid image_data encoding".to_string()))?;
                let result = self.upload.upload_image("conference.jpg", "image/jpeg", bytes).await?;
                req.image_url = Some(result.s3_url);
            }
        }

        self.validate_create(&req)?;

        let item = self.dao.create_conference(&req, auth.user_id).await?;

        // Send "conference created" email to all parents in selected classrooms (fire-and-forget)
        let conf_id = item.id;
        let title = item.title.clone();
        let date_str = item.conference_date.format("%B %d, %Y").to_string();
        let description = item.description.clone();
        let school_name = self.dao.get_school_name(auth.school_id).await
            .unwrap_or_else(|_| "The Goddard School".to_string());
        let base = parent_dashboard_url();
        let book_url = if let Some(ref sub) = req.school_subdomain {
            format!("{}/{}/dashboard/conferences/{}", base.trim_end_matches('/'), sub, conf_id)
        } else {
            format!("{}/dashboard/conferences/{}", base.trim_end_matches('/'), conf_id)
        };
        // Send emails inline — tokio::spawn gets frozen by Lambda before completing
        let image_url_for_email = item.image_url.clone();
        if let Ok(parents) = self.dao.list_parents_for_conference(conf_id).await {
            println!("[Conference] Sending emails to {} parent rows", parents.len());
            let mut grouped: std::collections::HashMap<String, (String, Vec<String>)> =
                std::collections::HashMap::new();
            for p in parents {
                if p.parent_email.is_empty() { continue; }
                let entry = grouped.entry(p.parent_email.clone())
                    .or_insert_with(|| (p.parent_name.clone(), Vec::new()));
                if !entry.1.contains(&p.child_name) {
                    entry.1.push(p.child_name.clone());
                }
            }
            println!("[Conference] Grouped into {} unique parent emails", grouped.len());
            for (email, (parent_name, child_names)) in grouped {
                let html = email_templates::conference_created_html(
                    &parent_name,
                    &title,
                    &date_str,
                    &description,
                    &school_name,
                    &child_names,
                    image_url_for_email.as_deref(),
                    &book_url,
                );
                let result = self.email.dispatch(
                    &email,
                    &format!("Conference Scheduled: {}", title),
                    &html,
                ).await;
                if let Err(e) = result {
                    println!("[Conference] Email error for {}: {:?}", email, e);
                }
            }
        } else {
            println!("[Conference] Failed to fetch parents for conference {}", conf_id);
        }

        Ok(item)
    }

    // ── Admin: list ───────────────────────────────────────────────────────────

    pub async fn list_conferences(
        &self,
        school_id: Uuid,
        auth: &AuthContext,
    ) -> ApiResult<Vec<ConferenceListItem>> {
        let sid = if matches!(auth.role, UserRole::SuperAdmin) { school_id } else { auth.school_id };
        self.dao.list_conferences(sid).await
    }

    // ── Admin: stats ──────────────────────────────────────────────────────────

    pub async fn get_conference_stats(
        &self,
        school_id: Uuid,
        auth: &AuthContext,
    ) -> ApiResult<ConferenceStats> {
        let sid = if matches!(auth.role, UserRole::SuperAdmin) { school_id } else { auth.school_id };
        self.dao.get_conference_stats(sid).await
    }

    // ── Detail ────────────────────────────────────────────────────────────────

    pub async fn get_conference_detail(&self, id: Uuid) -> ApiResult<ConferenceDetail> {
        self.dao.get_conference_detail(id).await?
            .ok_or_else(|| AppError::NotFound("Conference not found".to_string()))
    }

    // ── Update ────────────────────────────────────────────────────────────────

    pub async fn update_conference(
        &self,
        mut req: UpdateConferenceRequest,
        auth: &AuthContext,
    ) -> ApiResult<ConferenceListItem> {
        if let Some(date) = req.conference_date {
            if date < Utc::now().date_naive() {
                return Err(AppError::Validation("Conference date cannot be in the past".to_string()));
            }
        }
        if let Some(classrooms) = &req.classrooms {
            if classrooms.is_empty() {
                return Err(AppError::Validation("At least one classroom is required".to_string()));
            }
            for cr in classrooms {
                if cr.slots.is_empty() {
                    return Err(AppError::Validation("Each classroom must have at least one time slot".to_string()));
                }
                self.validate_slots(&cr.slots)?;
            }
        }
        let _ = auth;
        self.dao.update_conference(&req).await
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    pub async fn delete_conference(&self, id: Uuid, _auth: &AuthContext) -> ApiResult<()> {
        let booked = self.dao.get_conference_booking_count(id).await?;
        if booked > 0 {
            return Err(AppError::Conflict(format!(
                "Cannot delete: this conference has {} active booking(s). Cancel all bookings first.",
                booked
            )));
        }
        self.dao.delete_conference(id).await
    }

    // ── Report ────────────────────────────────────────────────────────────────

    pub async fn get_conference_report(
        &self,
        id: Uuid,
        _auth: &AuthContext,
    ) -> ApiResult<ConferenceReportResponse> {
        self.dao.get_conference_report(id).await?
            .ok_or_else(|| AppError::NotFound("Conference not found".to_string()))
    }

    // ── Reminders ─────────────────────────────────────────────────────────────

    pub async fn send_reminders(&self, conference_id: Uuid, parent_email: Option<&str>, auth: &AuthContext) -> ApiResult<usize> {
        let conf = self.dao.get_conference_detail(conference_id).await?
            .ok_or_else(|| AppError::NotFound("Conference not found".to_string()))?;

        let school_name = self.dao.get_school_name(auth.school_id).await
            .unwrap_or_else(|_| "The Goddard School".to_string());
        let date_str = conf.conference.conference_date.format("%B %d, %Y").to_string();
        let deadline_str = conf.conference.booking_close_at
            .map(|dt| dt.format("%B %d, %Y").to_string());
        let book_url = parent_dashboard_url();
        let title = conf.conference.title.clone();

        let unbooked = self.dao.list_all_unbooked_parents(conference_id).await?;
        let targets: Vec<_> = match parent_email {
            Some(email) => unbooked.iter().filter(|p| p.parent_email == email).collect(),
            None => unbooked.iter().collect(),
        };
        let count = targets.len();

        for p in targets {
            if p.parent_email.is_empty() { continue; }
            let html = email_templates::conference_reminder_html(
                &title,
                &date_str,
                deadline_str.as_deref(),
                &school_name,
                &book_url,
            );
            let _ = self.email.dispatch(
                &p.parent_email,
                &format!("Reminder: Book Your Conference Slot – {}", title),
                &html,
            ).await;
        }

        Ok(count)
    }

    // ── Parent: available conferences ─────────────────────────────────────────

    pub async fn list_available_for_parent(
        &self,
        school_id: Uuid,
        child_id: Option<Uuid>,
        auth: &AuthContext,
    ) -> ApiResult<Vec<ConferenceListItem>> {
        self.dao.list_available_for_parent(auth.user_id, school_id, child_id).await
    }

    // ── Parent: slots for a classroom ─────────────────────────────────────────

    pub async fn get_slots_for_classroom(
        &self,
        conference_id: Uuid,
        classroom_id: Uuid,
        auth: &AuthContext,
    ) -> ApiResult<Vec<SlotDetail>> {
        self.dao.get_slots_for_classroom(conference_id, classroom_id, Some(auth.user_id)).await
    }

    // ── Parent: book ──────────────────────────────────────────────────────────

    pub async fn book_slot(
        &self,
        req: BookSlotRequest,
        auth: &AuthContext,
    ) -> ApiResult<BookingListItem> {
        // Get classroom_id from slot
        let classroom_id = self.dao.get_slot_classroom(req.slot_id).await?;

        // Check no existing active booking for this child+classroom+conference
        if let Some(_existing) = self.dao.find_existing_booking(req.conference_id, classroom_id, req.child_id).await? {
            return Err(AppError::Conflict(
                "You already have a booking for this classroom in this conference".to_string()
            ));
        }

        // Check slot capacity
        let (capacity, booked_count) = self.dao.check_slot_capacity(req.slot_id).await?;
        if booked_count >= capacity as i64 {
            return Err(AppError::Conflict("This time slot is fully booked. Please choose another slot.".to_string()));
        }

        let booking = self.dao.create_booking(&req, auth.user_id).await?;

        // Send confirmation email
        let conf = self.dao.get_conference_detail(req.conference_id).await?;
        if let Some(conf) = conf {
            let school_name = self.dao.get_school_name(auth.school_id).await
                .unwrap_or_else(|_| "The Goddard School".to_string());
            let date_str = conf.conference.conference_date.format("%B %d, %Y").to_string();
            let time_str = format!("{} – {}", booking.slot_start, booking.slot_end);
            let html = email_templates::conference_booking_confirmed_html(
                &conf.conference.title,
                &date_str,
                &time_str,
                &booking.child_name,
                &booking.classroom_name,
                &school_name,
                &parent_dashboard_url(),
            );
            let _ = self.email.dispatch(
                &auth.email,
                &format!("Booking Confirmed – {}", conf.conference.title),
                &html,
            ).await;
        }

        Ok(booking)
    }

    // ── Parent: list my bookings ──────────────────────────────────────────────

    pub async fn list_my_bookings(
        &self,
        school_id: Uuid,
        auth: &AuthContext,
    ) -> ApiResult<Vec<BookingListItem>> {
        self.dao.list_my_bookings(auth.user_id, school_id).await
    }

    // ── Parent: change slot ───────────────────────────────────────────────────

    pub async fn update_booking(
        &self,
        booking_id: Uuid,
        req: UpdateBookingRequest,
        auth: &AuthContext,
    ) -> ApiResult<BookingListItem> {
        let (capacity, booked_count) = self.dao.check_slot_capacity(req.slot_id).await?;
        if booked_count >= capacity as i64 {
            return Err(AppError::Conflict("The selected slot is fully booked. Please choose another.".to_string()));
        }

        let booking = self.dao.update_booking(booking_id, req.slot_id, auth.user_id).await?;

        // Send updated confirmation
        if let Ok(conf_opt) = self.dao.get_conference_detail(booking.conference_id).await {
            if let Some(conf) = conf_opt {
                let school_name = self.dao.get_school_name(auth.school_id).await
                    .unwrap_or_else(|_| "The Goddard School".to_string());
                let date_str = conf.conference.conference_date.format("%B %d, %Y").to_string();
                let time_str = format!("{} – {}", booking.slot_start, booking.slot_end);
                let html = email_templates::conference_booking_confirmed_html(
                    &conf.conference.title, &date_str, &time_str,
                    &booking.child_name, &booking.classroom_name,
                    &school_name, &parent_dashboard_url(),
                );
                let _ = self.email.dispatch(
                    &auth.email,
                    &format!("Booking Updated – {}", conf.conference.title),
                    &html,
                ).await;
            }
        }

        Ok(booking)
    }

    // ── Parent: cancel ────────────────────────────────────────────────────────

    pub async fn cancel_booking(&self, booking_id: Uuid, auth: &AuthContext) -> ApiResult<()> {
        self.dao.cancel_booking(booking_id, auth.user_id).await
    }

    // ── Validation helpers ────────────────────────────────────────────────────

    fn validate_create(&self, req: &CreateConferenceRequest) -> ApiResult<()> {
        if req.title.trim().is_empty() {
            return Err(AppError::Validation("Conference title cannot be empty".to_string()));
        }
        if req.description.trim().is_empty() {
            return Err(AppError::Validation("Description cannot be empty".to_string()));
        }
        if req.conference_date < Utc::now().date_naive() {
            return Err(AppError::Validation("Conference date cannot be in the past".to_string()));
        }
        if let Some(close) = req.booking_close_at {
            if close < Utc::now() {
                return Err(AppError::Validation("Booking deadline cannot be in the past".to_string()));
            }
        }
        if req.classrooms.is_empty() {
            return Err(AppError::Validation("At least one classroom is required".to_string()));
        }
        for cr in &req.classrooms {
            if cr.slots.is_empty() {
                return Err(AppError::Validation("Each classroom must have at least one time slot".to_string()));
            }
            self.validate_slots(&cr.slots)?;
        }
        Ok(())
    }

    fn validate_slots(&self, slots: &[crate::models::conference::SlotInput]) -> ApiResult<()> {
        for s in slots {
            let start: chrono::NaiveTime = s.start_time.parse()
                .map_err(|_| AppError::Validation(format!("Invalid start_time: {}", s.start_time)))?;
            let end: chrono::NaiveTime = s.end_time.parse()
                .map_err(|_| AppError::Validation(format!("Invalid end_time: {}", s.end_time)))?;
            if end <= start {
                return Err(AppError::Validation(
                    format!("end_time ({}) must be after start_time ({})", s.end_time, s.start_time)
                ));
            }
            if s.capacity < 1 {
                return Err(AppError::Validation("Slot capacity must be at least 1".to_string()));
            }
        }
        Ok(())
    }
}
