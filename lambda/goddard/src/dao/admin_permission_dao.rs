use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::{AppError, ApiResult};
use crate::middleware::auth::AuthContext;
use crate::models::schema::UserRole;

#[derive(Clone)]
pub struct AdminPermissionDao {
    pool: Pool,
}

impl AdminPermissionDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// SuperAdmin always passes; Admin must have the permission in users.permissions.
    pub async fn ensure_permission(&self, auth: &AuthContext, permission: &str) -> ApiResult<()> {
        if matches!(auth.role, UserRole::SuperAdmin) {
            return Ok(());
        }
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "SELECT 1 FROM users WHERE id = $1 AND permissions @> $2::jsonb AND (is_active = true OR is_active IS NULL)",
            &[&auth.user_id, &serde_json::json!([permission])],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        row.map(|_| ()).ok_or_else(|| {
            AppError::Authorization("You don't have permission to access this feature".to_string())
        })
    }

    pub async fn set_permissions(&self, user_id: Uuid, permissions: Vec<String>) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let perms_json = serde_json::json!(permissions);
        client.execute(
            "UPDATE users SET permissions = $2 WHERE id = $1",
            &[&user_id, &perms_json],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
