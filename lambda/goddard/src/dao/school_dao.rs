use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use crate::{
    error::{AppError, ApiResult},
    models::school::{School, CreateSchoolRequest, UpdateSchoolRequest},
};

pub struct SchoolDao {
    pool: Pool,
}

impl SchoolDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_school(row: &Row) -> School {
        School {
            id: row.get("id"),
            name: row.get("name"),
            subdomain: row.get("subdomain"),
            settings: row.get("settings"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn create_school(&self, request: &CreateSchoolRequest) -> ApiResult<School> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            INSERT INTO schools (id, name, subdomain, settings, is_active, created_at)
            VALUES (gen_random_uuid(), $1, $2, $3, true, NOW())
            RETURNING id, name, subdomain, settings, is_active, created_at, updated_at
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let row = client.query_one(&stmt, &[&request.name, &request.subdomain, &request.settings])
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::row_to_school(&row))
    }

    pub async fn get_all_schools(&self) -> ApiResult<Vec<School>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            SELECT id, name, subdomain, settings, is_active, created_at, updated_at
            FROM schools
            WHERE (is_active = true OR is_active IS NULL)
            ORDER BY created_at DESC
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = client.query(&stmt, &[])
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let schools = rows.iter().map(Self::row_to_school).collect();
        Ok(schools)
    }

    pub async fn get_school_by_id(&self, school_id: &Uuid) -> ApiResult<Option<School>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            SELECT id, name, subdomain, settings, is_active, created_at, updated_at
            FROM schools
            WHERE id = $1 AND (is_active = true OR is_active IS NULL)
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = client.query(&stmt, &[school_id])
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::row_to_school(&rows[0])))
        }
    }

    pub async fn update_school(&self, request: &UpdateSchoolRequest) -> ApiResult<School> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Build the SQL query with escaped values to avoid prepared statement conflicts
        let query = format!(
            r#"
            UPDATE schools
            SET name = '{}',
                subdomain = '{}',
                updated_at = NOW()
            WHERE id = '{}' AND (is_active = true OR is_active IS NULL)
            RETURNING id, name, subdomain, settings, is_active, created_at, updated_at
            "#,
            request.name.replace('\'', "''"),
            request.subdomain.replace('\'', "''"),
            request.id
        );

        // Use simple_query to avoid prepared statements entirely
        let result = client.simple_query(&query).await
            .map_err(|e| AppError::Database(format!("Failed to update school: {}", e)))?;

        // Parse the result from simple_query
        for message in result {
            if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                // Extract values from SimpleQueryRow by index
                let school = School {
                    id: row.get(0).unwrap().parse().unwrap(),
                    name: row.get(1).unwrap().to_string(),
                    subdomain: row.get(2).unwrap().to_string(),
                    settings: row.get(3).and_then(|s| serde_json::from_str(s).ok()),
                    is_active: row.get(4).and_then(|s| s.parse().ok()),
                    created_at: row.get(5).and_then(|s| s.parse().ok()),
                    updated_at: row.get(6).and_then(|s| s.parse().ok()),
                };
                return Ok(school);
            }
        }
        Err(AppError::NotFound("School not found".to_string()))
    }

    pub async fn delete_school(&self, school_id: &Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            UPDATE schools
            SET is_active = false, updated_at = NOW()
            WHERE id = $1
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows_affected = client.execute(&stmt, &[school_id])
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("School not found".to_string()));
        }

        Ok(())
    }

    pub async fn check_subdomain_exists(&self, subdomain: &str, exclude_id: Option<&Uuid>) -> ApiResult<bool> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let count: i64 = if let Some(exclude_id) = exclude_id {
            {
                let stmt = client.prepare(
                    "SELECT COUNT(*) FROM schools WHERE subdomain = $1 AND id != $2 AND (is_active = true OR is_active IS NULL)"
                ).await
                .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;
                client.query_one(&stmt, &[&subdomain, exclude_id])
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?
                    .get(0)
            }
        } else {
            {
                let stmt = client.prepare(
                    "SELECT COUNT(*) FROM schools WHERE subdomain = $1 AND (is_active = true OR is_active IS NULL)"
                ).await
                .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;
                client.query_one(&stmt, &[&subdomain])
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?
                    .get(0)
            }
        };

        Ok(count > 0)
    }
}