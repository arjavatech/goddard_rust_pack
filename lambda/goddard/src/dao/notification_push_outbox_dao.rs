use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::notification_push_outbox::NotificationPushJob;

pub struct NotificationPushOutboxDao {
    pool: Pool,
}

impl NotificationPushOutboxDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Atomically lease a bounded set of ready rows. SKIP LOCKED allows future
    /// worker invocations to run safely in parallel.
    pub async fn claim_ready(&self, limit: i64) -> Result<Vec<NotificationPushJob>, AppError> {
        let mut client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        let tx = client.transaction().await.map_err(|e| {
            AppError::Database(format!("Failed to start push queue transaction: {}", e))
        })?;
        let rows = tx
            .query(
                r#"
            WITH ready AS (
                SELECT id
                FROM notification_push_outbox
                WHERE (status = 'pending' AND next_attempt_at <= NOW())
                   OR (status = 'processing' AND locked_until <= NOW())
                ORDER BY next_attempt_at ASC, created_at ASC
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            UPDATE notification_push_outbox o
            SET status = 'processing', attempts = o.attempts + 1,
                locked_until = NOW() + INTERVAL '5 minutes', updated_at = NOW()
            FROM ready r, notifications n
            WHERE o.id = r.id AND n.id = o.notification_id
            RETURNING o.id, o.notification_id, o.user_id, o.device_token,
                      n.notification_type, n.title, n.body, n.action_url
            "#,
                &[&limit],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to claim push jobs: {}", e)))?;
        tx.commit()
            .await
            .map_err(|e| AppError::Database(format!("Failed to commit push claim: {}", e)))?;
        Ok(rows.iter().map(Self::row_to_job).collect())
    }

    pub async fn mark_sent(&self, id: Uuid) -> Result<(), AppError> {
        self.finish(id, "sent", None, None).await
    }

    pub async fn mark_terminal_failure(&self, id: Uuid, error: &str) -> Result<(), AppError> {
        self.finish(id, "failed", Some(error), None).await
    }

    pub async fn retry_later(&self, id: Uuid, error: &str) -> Result<(), AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        // Exponential backoff (30s, 60s, 120s ... capped at one hour). Attempts
        // was incremented while claiming, so it is safe to derive the delay in SQL.
        client.execute(
            r#"
            UPDATE notification_push_outbox
            SET status = CASE WHEN attempts >= 8 THEN 'failed' ELSE 'pending' END,
                last_error = $2,
                locked_until = NULL,
                next_attempt_at = NOW() + (LEAST(3600, 30 * POWER(2, GREATEST(attempts - 1, 0))) * INTERVAL '1 second'),
                updated_at = NOW()
            WHERE id = $1
            "#,
            &[&id, &error],
        ).await.map_err(|e| AppError::Database(format!("Failed to retry push job: {}", e)))?;
        Ok(())
    }

    async fn finish(
        &self,
        id: Uuid,
        status: &str,
        error: Option<&str>,
        sent_at: Option<bool>,
    ) -> Result<(), AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        let is_sent = sent_at.unwrap_or(status == "sent");
        client
            .execute(
                r#"
            UPDATE notification_push_outbox
            SET status = $2, last_error = $3, locked_until = NULL,
                sent_at = CASE WHEN $4 THEN NOW() ELSE sent_at END,
                updated_at = NOW()
            WHERE id = $1
            "#,
                &[&id, &status, &error, &is_sent],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to finish push job: {}", e)))?;
        Ok(())
    }

    fn row_to_job(row: &Row) -> NotificationPushJob {
        NotificationPushJob {
            id: row.get("id"),
            notification_id: row.get("notification_id"),
            user_id: row.get("user_id"),
            device_token: row.get("device_token"),
            notification_type: row.get("notification_type"),
            title: row.get("title"),
            body: row.get("body"),
            action_url: row.get("action_url"),
        }
    }
}
