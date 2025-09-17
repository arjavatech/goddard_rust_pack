use sqlx::PgPool;
use uuid::Uuid;
use crate::{
    error::{AppError, ApiResult},
    models::school::{School, CreateSchoolRequest, UpdateSchoolRequest},
};

pub struct SchoolDao {
    pool: PgPool,
}

impl SchoolDao {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_school(&self, request: &CreateSchoolRequest) -> ApiResult<School> {
        let school = sqlx::query_as!(
            School,
            r#"
            INSERT INTO schools (id, name, subdomain, settings, is_active, created_at)
            VALUES (gen_random_uuid(), $1, $2, $3, true, NOW())
            RETURNING id, name, subdomain, settings, is_active, created_at as "created_at!", updated_at
            "#,
            request.name,
            request.subdomain,
            request.settings
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(school)
    }

    pub async fn get_all_schools(&self) -> ApiResult<Vec<School>> {
        let schools = sqlx::query_as!(
            School,
            r#"
            SELECT id, name, subdomain, settings, is_active, created_at as "created_at!", updated_at
            FROM schools
            WHERE (is_active = true OR is_active IS NULL)
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(schools)
    }

    pub async fn get_school_by_id(&self, school_id: &Uuid) -> ApiResult<Option<School>> {
        let school = sqlx::query_as!(
            School,
            r#"
            SELECT id, name, subdomain, settings, is_active, created_at as "created_at!", updated_at
            FROM schools
            WHERE id = $1 AND (is_active = true OR is_active IS NULL)
            "#,
            school_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(school)
    }

    pub async fn update_school(&self, request: &UpdateSchoolRequest) -> ApiResult<School> {
        let school = sqlx::query_as!(
            School,
            r#"
            UPDATE schools
            SET name = $2,
                subdomain = $3,
                settings = $4,
                updated_at = NOW()
            WHERE id = $1 AND (is_active = true OR is_active IS NULL)
            RETURNING id, name, subdomain, settings, is_active, created_at as "created_at!", updated_at
            "#,
            request.id,
            request.name,
            request.subdomain,
            request.settings
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(school)
    }

    pub async fn delete_school(&self, school_id: &Uuid) -> ApiResult<()> {
        let result = sqlx::query!(
            r#"
            UPDATE schools
            SET is_active = false, updated_at = NOW()
            WHERE id = $1
            "#,
            school_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("School not found".to_string()));
        }

        Ok(())
    }

    pub async fn check_subdomain_exists(&self, subdomain: &str, exclude_id: Option<&Uuid>) -> ApiResult<bool> {
        let count = if let Some(exclude_id) = exclude_id {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM schools WHERE subdomain = $1 AND id != $2 AND (is_active = true OR is_active IS NULL)",
                subdomain,
                exclude_id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM schools WHERE subdomain = $1 AND (is_active = true OR is_active IS NULL)",
                subdomain
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        Ok(count.unwrap_or(0) > 0)
    }
}