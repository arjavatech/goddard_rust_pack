use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::AppError;

pub struct DeviceTokenDao {
    pool: Pool,
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

        Ok(rows.into_iter().map(|r| r.get::<_, String>("token")).collect())
    }

    /// Batched fetch used by admin fan-out — one query for N user_ids returning
    /// (user_id, token) tuples.
    pub async fn tokens_for_users(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get db connection: {}", e)))?;

        let rows = client
            .query(
                "SELECT user_id, token FROM device_tokens WHERE user_id = ANY($1)",
                &[&user_ids],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to load device tokens: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| (r.get::<_, Uuid>("user_id"), r.get::<_, String>("token")))
            .collect())
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
    pub async fn delete_token_for_user(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<(), AppError> {
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
