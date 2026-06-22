use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::notification::{
    CreateNotification, Notification, NotificationFilter,
};

pub struct NotificationDao {
    pool: Pool,
}

impl NotificationDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert a single notification for a specific user.
    pub async fn insert_one(
        &self,
        user_id: Uuid,
        payload: &CreateNotification,
    ) -> Result<Notification, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        let row = client
            .query_one(
                r#"
                INSERT INTO notifications (
                    user_id, school_id, notification_type, title, body,
                    related_entity_id, related_entity_type, action_url
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id, user_id, school_id, notification_type, title, body,
                          related_entity_id, related_entity_type, action_url,
                          is_read, read_at, created_at
                "#,
                &[
                    &user_id,
                    &payload.school_id,
                    &payload.notification_type,
                    &payload.title,
                    &payload.body,
                    &payload.related_entity_id,
                    &payload.related_entity_type,
                    &payload.action_url,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to insert notification: {}", e)))?;

        Ok(Self::row_to_notification(&row))
    }

    /// Fan out one notification per active admin / superadmin of the given school in a
    /// single INSERT … SELECT round-trip. Returns the number of rows inserted.
    pub async fn insert_many_for_school_admins(
        &self,
        school_id: Uuid,
        payload: &CreateNotification,
        exclude_user_id: Option<Uuid>,
    ) -> Result<u64, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        let inserted = client
            .execute(
                r#"
                INSERT INTO notifications (
                    user_id, school_id, notification_type, title, body,
                    related_entity_id, related_entity_type, action_url
                )
                SELECT u.id, $1, $2, $3, $4, $5, $6, $7
                FROM users u
                WHERE u.school_id = $1
                  AND u.role IN ('Admin', 'SuperAdmin')
                  AND COALESCE(u.is_active, true) = true
                  AND ($8::uuid IS NULL OR u.id <> $8)
                "#,
                &[
                    &school_id,
                    &payload.notification_type,
                    &payload.title,
                    &payload.body,
                    &payload.related_entity_id,
                    &payload.related_entity_type,
                    &payload.action_url,
                    &exclude_user_id,
                ],
            )
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to fan out admin notifications: {}", e))
            })?;

        Ok(inserted)
    }

    /// Page through a user's notifications. Returns items + total + unread_count in two
    /// queries (one filtered list, one aggregate).
    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: NotificationFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Notification>, i64, i64), AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        let (where_clause, is_read_filter): (&str, Option<bool>) = match filter {
            NotificationFilter::All => ("user_id = $1", None),
            NotificationFilter::Unread => ("user_id = $1 AND is_read = false", Some(false)),
            NotificationFilter::Read => ("user_id = $1 AND is_read = true", Some(true)),
        };

        let list_sql = format!(
            "SELECT id, user_id, school_id, notification_type, title, body, \
             related_entity_id, related_entity_type, action_url, is_read, read_at, created_at \
             FROM notifications WHERE {} ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            where_clause
        );

        let list_rows = client
            .query(list_sql.as_str(), &[&user_id, &limit, &offset])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list notifications: {}", e)))?;

        let items: Vec<Notification> = list_rows.iter().map(Self::row_to_notification).collect();

        // total = total matching the filter; unread_count is always the count of unread for
        // the user regardless of filter (so the bell badge stays accurate).
        let total: i64 = match is_read_filter {
            None => {
                let r = client
                    .query_one(
                        "SELECT COUNT(*)::bigint FROM notifications WHERE user_id = $1",
                        &[&user_id],
                    )
                    .await
                    .map_err(|e| AppError::Database(format!("Failed to count: {}", e)))?;
                r.get(0)
            }
            Some(flag) => {
                let r = client
                    .query_one(
                        "SELECT COUNT(*)::bigint FROM notifications WHERE user_id = $1 AND is_read = $2",
                        &[&user_id, &flag],
                    )
                    .await
                    .map_err(|e| AppError::Database(format!("Failed to count: {}", e)))?;
                r.get(0)
            }
        };

        let unread_count = self.count_unread(user_id).await?;

        Ok((items, total, unread_count))
    }

    pub async fn count_unread(&self, user_id: Uuid) -> Result<i64, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        let row = client
            .query_one(
                "SELECT COUNT(*)::bigint FROM notifications WHERE user_id = $1 AND is_read = false",
                &[&user_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to count unread: {}", e)))?;
        Ok(row.get(0))
    }

    /// Mark a single notification as read. Scoped by user_id to prevent cross-user access.
    /// Returns true if a row was updated, false if no matching unread notification existed.
    pub async fn mark_read(
        &self,
        notification_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        let updated = client
            .execute(
                "UPDATE notifications SET is_read = true, read_at = NOW() \
                 WHERE id = $1 AND user_id = $2 AND is_read = false",
                &[&notification_id, &user_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to mark read: {}", e)))?;
        Ok(updated > 0)
    }

    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        let updated = client
            .execute(
                "UPDATE notifications SET is_read = true, read_at = NOW() \
                 WHERE user_id = $1 AND is_read = false",
                &[&user_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to mark all read: {}", e)))?;
        Ok(updated)
    }

    fn row_to_notification(row: &Row) -> Notification {
        // created_at / read_at are TIMESTAMPTZ → DateTime<Utc> via tokio-postgres' chrono mapping.
        Notification {
            id: row.get("id"),
            user_id: row.get("user_id"),
            school_id: row.get("school_id"),
            notification_type: row.get("notification_type"),
            title: row.get("title"),
            body: row.get("body"),
            related_entity_id: row.get("related_entity_id"),
            related_entity_type: row.get("related_entity_type"),
            action_url: row.get("action_url"),
            is_read: row.get("is_read"),
            read_at: row.get::<_, Option<DateTime<Utc>>>("read_at"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
        }
    }
}
