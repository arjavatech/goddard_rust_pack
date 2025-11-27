use deadpool_postgres::Pool;
use tokio_postgres::Row;
use chrono::{DateTime, Utc};
use crate::error::{AppError, ApiResult};

#[derive(Debug, Clone)]
pub struct AuthUserStatus {
    pub id: Option<String>,
    pub email: Option<String>,
    pub invited_at: Option<DateTime<Utc>>,
    pub confirmation_sent_at: Option<DateTime<Utc>>,
    pub email_confirmed_at: Option<DateTime<Utc>>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthStats {
    pub total_users: i64,
    pub confirmed_users: i64,
    pub invited_not_confirmed: i64,
    pub confirmation_sent_not_confirmed: i64,
    pub users_who_signed_in: i64,
}

#[derive(Debug, Clone)]
pub struct UserDetails {
    pub id: uuid::Uuid,
    pub school_id: uuid::Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
    pub is_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct AuthDao {
    pool: Pool,
}

impl AuthDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_auth_stats(row: &Row) -> AuthStats {
        AuthStats {
            total_users: row.get::<_, i64>("total_users"),
            confirmed_users: row.get::<_, i64>("confirmed_users"),
            invited_not_confirmed: row.get::<_, i64>("invited_not_confirmed"),
            confirmation_sent_not_confirmed: row.get::<_, i64>("confirmation_sent_not_confirmed"),
            users_who_signed_in: row.get::<_, i64>("users_who_signed_in"),
        }
    }

    fn row_to_auth_user_status(row: &Row) -> AuthUserStatus {
        AuthUserStatus {
            id: row.get("id"),
            email: row.get("email"),
            invited_at: row.get("invited_at"),
            confirmation_sent_at: row.get("confirmation_sent_at"),
            email_confirmed_at: row.get("email_confirmed_at"),
            last_sign_in_at: row.get("last_sign_in_at"),
            created_at: row.get("created_at"),
            status: row.get("status"),
        }
    }

    pub async fn get_auth_verification_stats(&self) -> ApiResult<AuthStats> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            SELECT
                COUNT(*) as total_users,
                COUNT(CASE WHEN email_confirmed_at IS NOT NULL THEN 1 END) as confirmed_users,
                COUNT(CASE WHEN invited_at IS NOT NULL AND email_confirmed_at IS NULL THEN 1 END) as invited_not_confirmed,
                COUNT(CASE WHEN confirmation_sent_at IS NOT NULL AND email_confirmed_at IS NULL THEN 1 END) as confirmation_sent_not_confirmed,
                COUNT(CASE WHEN last_sign_in_at IS NOT NULL THEN 1 END) as users_who_signed_in
            FROM auth.users
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let row = client.query_one(&stmt, &[])
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::row_to_auth_stats(&row))
    }

    pub async fn get_user_details(&self, _school_id: Option<String>) -> ApiResult<Vec<AuthUserStatus>> {
        // TODO: Implement with tokio-postgres
        Ok(vec![])
    }

    pub async fn get_invitation_summary_by_role(&self) -> ApiResult<(i64, i64, i64, i64)> {
        // This would join with public.users to get role information
        // For now, returning mock data as we need the public.users integration
        Ok((0, 0, 0, 1)) // (super_admin, admin, teacher, parent)
    }

    pub async fn user_exists_by_email(&self, _email: &str) -> ApiResult<bool> {
        // TODO: Implement with tokio-postgres
        Ok(false)
    }

    pub async fn user_needs_confirmation(&self, _email: &str) -> ApiResult<bool> {
        // TODO: Implement with tokio-postgres
        Ok(true)
    }

    pub async fn update_confirmation_sent_at(&self, _email: &str) -> ApiResult<()> {
        // TODO: Implement with tokio-postgres
        Ok(())
    }

    pub async fn get_parent_id_by_auth_user_id(&self, auth_user_id: &uuid::Uuid) -> ApiResult<Option<uuid::Uuid>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT id FROM users WHERE auth_user_id = $1 AND role = 'Parent'";

        match client.query_opt(query, &[auth_user_id]).await {
            Ok(Some(row)) => {
                let parent_id: uuid::Uuid = row.get("id");
                Ok(Some(parent_id))
            },
            Ok(None) => Ok(None), // User not found or not a parent
            Err(e) => Err(AppError::Database(format!("Failed to query parent_id: {}", e))),
        }
    }

    /// Get all verified Admin users for a specific school
    pub async fn get_admins_by_school(&self, school_id: uuid::Uuid) -> ApiResult<Vec<UserDetails>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT id, school_id, first_name, last_name, email, role,
                   COALESCE(is_verified, false) as is_verified, created_at
            FROM users
            WHERE school_id = $1 AND role = 'Admin' AND is_verified = true AND (is_active = true OR is_active IS NULL)
            ORDER BY first_name, last_name
        "#;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to query admins: {}", e)))?;

        let mut admins = Vec::new();
        for row in rows {
            let created_at_naive: chrono::NaiveDateTime = row.get("created_at");
            let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at_naive, chrono::Utc);

            admins.push(UserDetails {
                id: row.get("id"),
                school_id: row.get("school_id"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                email: row.get("email"),
                role: row.get("role"),
                is_verified: row.get("is_verified"),
                created_at,
            });
        }

        Ok(admins)
    }

    /// Update admin user profile (first_name, last_name, phone_number)
    pub async fn update_admin_user(
        &self,
        user_id: uuid::Uuid,
        first_name: Option<String>,
        last_name: Option<String>,
        phone_number: Option<String>,
    ) -> ApiResult<UserDetails> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE users
            SET first_name = COALESCE($2, first_name),
                last_name = COALESCE($3, last_name),
                phone_number = COALESCE($4, phone_number),
                updated_at = NOW()
            WHERE id = $1 AND role = 'Admin' AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, first_name, last_name, email, role,
                      COALESCE(is_verified, false) as is_verified, created_at
        "#;

        match client.query_opt(query, &[&user_id, &first_name, &last_name, &phone_number]).await {
            Ok(Some(row)) => {
                let created_at_naive: chrono::NaiveDateTime = row.get("created_at");
                let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at_naive, chrono::Utc);

                Ok(UserDetails {
                    id: row.get("id"),
                    school_id: row.get("school_id"),
                    first_name: row.get("first_name"),
                    last_name: row.get("last_name"),
                    email: row.get("email"),
                    role: row.get("role"),
                    is_verified: row.get("is_verified"),
                    created_at,
                })
            },
            Ok(None) => Err(AppError::NotFound("Admin user not found or already deleted".to_string())),
            Err(e) => Err(AppError::Database(format!("Failed to update admin: {}", e))),
        }
    }

    /// Soft delete admin user (set is_active = false)
    pub async fn soft_delete_admin_user(&self, user_id: uuid::Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE users
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND role = 'Admin' AND (is_active = true OR is_active IS NULL)
        "#;

        let result = client.execute(query, &[&user_id]).await
            .map_err(|e| AppError::Database(format!("Failed to delete admin: {}", e)))?;

        if result == 0 {
            return Err(AppError::NotFound("Admin user not found or already deleted".to_string()));
        }

        Ok(())
    }

    pub async fn get_user_by_id(&self, user_id: uuid::Uuid) -> ApiResult<UserDetails> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT id, school_id, first_name, last_name, email, role, COALESCE(is_verified, false) as is_verified, created_at FROM users WHERE id = $1";

        match client.query_opt(query, &[&user_id]).await {
            Ok(Some(row)) => {
                let created_at_naive: chrono::NaiveDateTime = row.get("created_at");
                let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at_naive, chrono::Utc);

                Ok(UserDetails {
                    id: row.get("id"),
                    school_id: row.get("school_id"),
                    first_name: row.get("first_name"),
                    last_name: row.get("last_name"),
                    email: row.get("email"),
                    role: row.get("role"),
                    is_verified: row.get("is_verified"),
                    created_at,
                })
            },
            Ok(None) => Err(AppError::NotFound("User not found".to_string())),
            Err(e) => Err(AppError::Database(format!("Failed to query user: {}", e))),
        }
    }
}