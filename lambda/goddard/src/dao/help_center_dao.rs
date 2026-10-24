use crate::error::AppError;
use crate::models::help_center::{HelpCenterMessage, SubmitHelpMessageRequest};
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

#[derive(Clone)]
pub struct HelpCenterDao {
    pool: Pool,
}

impl HelpCenterDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn get_school_superadmin_email(&self, school_id: Uuid) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await.map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let rows = client
            .query(
                "SELECT email FROM users WHERE school_id = $1 AND role = 'SuperAdmin' \
                 AND (is_active = true OR is_active IS NULL) ORDER BY created_at ASC LIMIT 1",
                &[&school_id],
            )
            .await
            .map_err(|e: tokio_postgres::Error| AppError::Database(e.to_string()))?;
        Ok(rows.first().map(|r| r.get::<_, String>(0)))
    }

    pub async fn get_school_contact_email(&self, school_id: Uuid) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await.map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let rows = client
            .query(
                "SELECT settings->>'mail' FROM schools WHERE id = $1",
                &[&school_id],
            )
            .await
            .map_err(|e: tokio_postgres::Error| AppError::Database(e.to_string()))?;
        Ok(rows
            .first()
            .and_then(|r| r.get::<_, Option<String>>(0))
            .filter(|s| !s.is_empty()))
    }

    pub async fn create_message(
        &self,
        school_id: Uuid,
        sender_user_id: Option<Uuid>,
        sender_role: &str,
        req: &SubmitHelpMessageRequest,
        to_email: &str,
        from_email: &str,
    ) -> Result<HelpCenterMessage, AppError> {
        let client = self.pool.get().await.map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let rows = client
            .query(
                "INSERT INTO help_center_messages \
                   (school_id, sender_user_id, sender_role, sender_name, sender_email, \
                    subject, message, to_email, from_email) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
                 RETURNING id, school_id, sender_user_id, sender_role, sender_name, sender_email, \
                           subject, message, to_email, from_email, created_at",
                &[
                    &school_id,
                    &sender_user_id,
                    &sender_role,
                    &req.sender_name,
                    &req.sender_email,
                    &req.subject,
                    &req.message,
                    &to_email,
                    &from_email,
                ],
            )
            .await
            .map_err(|e: tokio_postgres::Error| AppError::Database(e.to_string()))?;

        let row = rows.first().ok_or_else(|| AppError::Internal("Insert returned no row".into()))?;
        Ok(HelpCenterMessage {
            id: row.get(0),
            school_id: row.get(1),
            sender_user_id: row.get(2),
            sender_role: row.get(3),
            sender_name: row.get(4),
            sender_email: row.get(5),
            subject: row.get(6),
            message: row.get(7),
            to_email: row.get(8),
            from_email: row.get(9),
            created_at: row.get::<_, DateTime<Utc>>(10),
        })
    }
}
