use sqlx::PgPool;
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

pub struct AuthDao {
    pool: PgPool,
}

impl AuthDao {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_auth_verification_stats(&self) -> ApiResult<AuthStats> {
        let stats = sqlx::query_as!(
            AuthStats,
            r#"
            SELECT
                COUNT(*) as "total_users!",
                COUNT(CASE WHEN email_confirmed_at IS NOT NULL THEN 1 END) as "confirmed_users!",
                COUNT(CASE WHEN invited_at IS NOT NULL AND email_confirmed_at IS NULL THEN 1 END) as "invited_not_confirmed!",
                COUNT(CASE WHEN confirmation_sent_at IS NOT NULL AND email_confirmed_at IS NULL THEN 1 END) as "confirmation_sent_not_confirmed!",
                COUNT(CASE WHEN last_sign_in_at IS NOT NULL THEN 1 END) as "users_who_signed_in!"
            FROM auth.users
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(stats)
    }

    pub async fn get_user_details(&self, school_id: Option<String>) -> ApiResult<Vec<AuthUserStatus>> {
        let mut query = r#"
            SELECT
                au.id::text,
                au.email,
                au.invited_at,
                au.confirmation_sent_at,
                au.email_confirmed_at,
                au.last_sign_in_at,
                au.created_at,
                CASE
                    WHEN au.email_confirmed_at IS NOT NULL THEN 'Confirmed'
                    WHEN au.confirmation_sent_at IS NOT NULL THEN 'Confirmation Email Sent'
                    WHEN au.invited_at IS NOT NULL THEN 'Invited'
                    ELSE 'Pending'
                END as status
            FROM auth.users au
        "#.to_string();

        // Add school filtering if needed
        if school_id.is_some() {
            query.push_str(r#"
                LEFT JOIN public.users pu ON au.email = pu.email
                WHERE pu.school_id = $1
            "#);
        }

        query.push_str(" ORDER BY au.created_at DESC");

        let users = if let Some(school_id) = school_id {
            let school_uuid = uuid::Uuid::parse_str(&school_id)
                .map_err(|_| AppError::Validation("Invalid school_id format".to_string()))?;

            sqlx::query_as!(
                AuthUserStatus,
                r#"
                SELECT
                    au.id::text,
                    au.email,
                    au.invited_at,
                    au.confirmation_sent_at,
                    au.email_confirmed_at,
                    au.last_sign_in_at,
                    au.created_at,
                    CASE
                        WHEN au.email_confirmed_at IS NOT NULL THEN 'Confirmed'
                        WHEN au.confirmation_sent_at IS NOT NULL THEN 'Confirmation Email Sent'
                        WHEN au.invited_at IS NOT NULL THEN 'Invited'
                        ELSE 'Pending'
                    END as status
                FROM auth.users au
                LEFT JOIN public.users pu ON au.email = pu.email
                WHERE pu.school_id = $1
                ORDER BY au.created_at DESC
                "#,
                school_uuid
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        } else {
            sqlx::query_as!(
                AuthUserStatus,
                r#"
                SELECT
                    au.id::text,
                    au.email,
                    au.invited_at,
                    au.confirmation_sent_at,
                    au.email_confirmed_at,
                    au.last_sign_in_at,
                    au.created_at,
                    CASE
                        WHEN au.email_confirmed_at IS NOT NULL THEN 'Confirmed'
                        WHEN au.confirmation_sent_at IS NOT NULL THEN 'Confirmation Email Sent'
                        WHEN au.invited_at IS NOT NULL THEN 'Invited'
                        ELSE 'Pending'
                    END as status
                FROM auth.users au
                ORDER BY au.created_at DESC
                "#
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        Ok(users)
    }

    pub async fn get_invitation_summary_by_role(&self) -> ApiResult<(i64, i64, i64, i64)> {
        // This would join with public.users to get role information
        // For now, returning mock data as we need the public.users integration
        Ok((0, 0, 0, 1)) // (super_admin, admin, teacher, parent)
    }

    pub async fn user_exists_by_email(&self, email: &str) -> ApiResult<bool> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM auth.users WHERE email = $1",
            email
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count.unwrap_or(0) > 0)
    }

    pub async fn user_needs_confirmation(&self, email: &str) -> ApiResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT email_confirmed_at
            FROM auth.users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        match result {
            Some(user) => Ok(user.email_confirmed_at.is_none()),
            None => Err(AppError::NotFound("User not found".to_string())),
        }
    }

    pub async fn update_confirmation_sent_at(&self, email: &str) -> ApiResult<()> {
        sqlx::query!(
            r#"
            UPDATE auth.users
            SET confirmation_sent_at = NOW()
            WHERE email = $1
            "#,
            email
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}