use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::AppError;

pub struct DeviceTokenDao {
    pool: Pool,
}

#[derive(serde::Serialize)]
pub struct DeviceTokenStatus {
    pub registered_devices: i64,
    pub web_devices: i64,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl DeviceTokenDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn upsert_token(
        &self,
        user_id: Uuid,
        token: &str,
        platform: &str,
        user_agent: Option<&str>,
    ) -> Result<(), AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        client
            .execute(
                r#"
                INSERT INTO device_tokens (user_id, token, platform, user_agent)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (token)
                DO UPDATE SET
                    user_id      = EXCLUDED.user_id,
                    platform     = EXCLUDED.platform,
                    user_agent   = EXCLUDED.user_agent,
                    last_seen_at = NOW()
                "#,
                &[&user_id, &token, &platform, &user_agent],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to upsert device token: {}", e)))?;

        Ok(())
    }

    pub async fn tokens_for_user(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        let rows = client
            .query(
                "SELECT token FROM device_tokens WHERE user_id = $1",
                &[&user_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to load device tokens: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| r.get::<_, String>("token"))
            .collect())
    }

    /// Safe registration diagnostics for the current user. Deliberately never
    /// returns bearer-like FCM tokens.
    pub async fn status_for_user(&self, user_id: Uuid) -> Result<DeviceTokenStatus, AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;
        let row = client
            .query_one(
                r#"
            SELECT COUNT(*)::BIGINT AS registered_devices,
                   COUNT(*) FILTER (WHERE platform = 'web')::BIGINT AS web_devices,
                   MAX(last_seen_at) AS last_seen_at
            FROM device_tokens
            WHERE user_id = $1
            "#,
                &[&user_id],
            )
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to load device token status: {}", e))
            })?;
        Ok(DeviceTokenStatus {
            registered_devices: row.get("registered_devices"),
            web_devices: row.get("web_devices"),
            last_seen_at: row.get("last_seen_at"),
        })
    }

    /// Unconditional delete — used when FCM returns UNREGISTERED for a token, since
    /// at that point the token is dead regardless of which user it belonged to.
    pub async fn delete_token(&self, token: &str) -> Result<(), AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        client
            .execute("DELETE FROM device_tokens WHERE token = $1", &[&token])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete device token: {}", e)))?;

        Ok(())
    }

    /// User-scoped delete — used by the logout endpoint so a stolen JWT can't wipe
    /// someone else's tokens.
    pub async fn delete_token_for_user(&self, token: &str, user_id: Uuid) -> Result<(), AppError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        client
            .execute(
                "DELETE FROM device_tokens WHERE token = $1 AND user_id = $2",
                &[&token, &user_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete device token: {}", e)))?;

        Ok(())
    }
}
