use deadpool_postgres::{Pool, Client};
use tokio_postgres::Row;
use uuid::Uuid;
use crate::{
    error::{AppError, ApiResult},
    models::school::{School, CreateSchoolRequest, UpdateSchoolRequest},
};
use std::time::Duration;

pub struct SchoolDao {
    pool: Pool,
}

impl SchoolDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // Helper function to get database connection with timeout
    async fn get_connection(&self) -> ApiResult<Client> {
        println!("[SchoolDao] Attempting to get database connection with 5s timeout");
        let timeout_duration = Duration::from_secs(5);
        let get_connection = self.pool.get();

        match tokio::time::timeout(timeout_duration, get_connection).await {
            Ok(Ok(client)) => {
                println!("[SchoolDao] Database connection acquired successfully");
                Ok(client)
            },
            Ok(Err(e)) => {
                println!("[SchoolDao] Failed to get connection from pool: {:?}", e);
                Err(AppError::Database(format!("Failed to get connection from pool: {}", e)))
            },
            Err(_) => {
                println!("[SchoolDao] Database connection timeout after 5s");
                Err(AppError::Database("Database connection timeout (5s) - database may be unreachable".to_string()))
            }
        }
    }

    // Execute query with automatic connection cleanup
    async fn execute_with_connection<T, F, Fut>(&self, operation: F) -> ApiResult<T>
    where
        F: FnOnce(Client) -> Fut,
        Fut: std::future::Future<Output = ApiResult<T>>,
    {
        let client = self.get_connection().await?;
        let result = operation(client).await;
        // Connection is automatically dropped here, returning to pool
        result
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
        println!("[SchoolDao] Starting create_school - getting database connection");

        let name = request.name.clone();
        let subdomain = request.subdomain.clone();
        let settings = request.settings.clone();

        self.execute_with_connection(|client| async move {
            println!("[SchoolDao] Database connection acquired successfully");
            println!("[SchoolDao] Executing INSERT query for school: name={}, subdomain={}", name, subdomain);

            let settings_json = match &settings {
                Some(s) => format!("'{}'", s.to_string().replace('\'', "''")),
                None => "NULL".to_string(),
            };

            let query = format!(
                r#"
                INSERT INTO schools (id, name, subdomain, settings, is_active, created_at)
                VALUES (gen_random_uuid(), '{}', '{}', {}, true, NOW())
                RETURNING id, name, subdomain, settings, is_active, created_at, updated_at
                "#,
                name.replace('\'', "''"),
                subdomain.replace('\'', "''"),
                settings_json
            );

            let result = client.simple_query(&query).await
                .map_err(|e| {
                    println!("[SchoolDao] INSERT query failed with error: {:?}", e);
                    AppError::Database(format!("Failed to create school: {}", e))
                })?;

            println!("[SchoolDao] INSERT query executed successfully");

            for message in result {
                if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                    let school = School {
                        id: row.get(0).unwrap().parse().unwrap(),
                        name: row.get(1).unwrap().to_string(),
                        subdomain: row.get(2).unwrap().to_string(),
                        settings: row.get(3).and_then(|s| serde_json::from_str(s).ok()),
                        is_active: row.get(4).and_then(|s| s.parse().ok()),
                        created_at: row.get(5).and_then(|s| s.parse().ok()),
                        updated_at: row.get(6).and_then(|s| s.parse().ok()),
                    };
                    println!("[SchoolDao] School object created successfully: id={}", school.id);
                    return Ok(school);
                }
            }

            Err(AppError::Database("No school returned from INSERT".to_string()))
        }).await
    }

    pub async fn get_all_schools(&self) -> ApiResult<Vec<School>> {
        self.execute_with_connection(|client| async move {
            let result = client.simple_query(
                r#"
                SELECT id, name, subdomain, settings, is_active, created_at, updated_at
                FROM schools
                WHERE (is_active = true OR is_active IS NULL)
                ORDER BY created_at DESC
                "#
            ).await
            .map_err(|e| AppError::Database(e.to_string()))?;

            let mut schools = Vec::new();
            for message in result {
                if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                    let school = School {
                        id: row.get(0).unwrap().parse().unwrap(),
                        name: row.get(1).unwrap().to_string(),
                        subdomain: row.get(2).unwrap().to_string(),
                        settings: row.get(3).and_then(|s| serde_json::from_str(s).ok()),
                        is_active: row.get(4).and_then(|s| s.parse().ok()),
                        created_at: row.get(5).and_then(|s| s.parse().ok()),
                        updated_at: row.get(6).and_then(|s| s.parse().ok()),
                    };
                    schools.push(school);
                }
            }
            Ok(schools)
        }).await
    }

    pub async fn get_school_by_id(&self, school_id: &Uuid) -> ApiResult<Option<School>> {
        let school_id = *school_id;

        self.execute_with_connection(|client| async move {
            let query = format!(
                r#"
                SELECT id, name, subdomain, settings, is_active, created_at, updated_at
                FROM schools
                WHERE id = '{}' AND (is_active = true OR is_active IS NULL)
                "#,
                school_id
            );

            let result = client.simple_query(&query).await
                .map_err(|e| AppError::Database(e.to_string()))?;

            for message in result {
                if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                    let school = School {
                        id: row.get(0).unwrap().parse().unwrap(),
                        name: row.get(1).unwrap().to_string(),
                        subdomain: row.get(2).unwrap().to_string(),
                        settings: row.get(3).and_then(|s| serde_json::from_str(s).ok()),
                        is_active: row.get(4).and_then(|s| s.parse().ok()),
                        created_at: row.get(5).and_then(|s| s.parse().ok()),
                        updated_at: row.get(6).and_then(|s| s.parse().ok()),
                    };
                    return Ok(Some(school));
                }
            }

            Ok(None)
        }).await
    }

    pub async fn update_school(&self, request: &UpdateSchoolRequest) -> ApiResult<School> {
        let client = self.get_connection().await?;

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
        let school_id = *school_id;

        self.execute_with_connection(|client| async move {
            let rows_affected = client.execute(
                r#"
                UPDATE schools
                SET is_active = false, updated_at = NOW()
                WHERE id = $1
                "#,
                &[&school_id]
            ).await
            .map_err(|e| AppError::Database(e.to_string()))?;

            if rows_affected == 0 {
                return Err(AppError::NotFound("School not found".to_string()));
            }

            Ok(())
        }).await
    }

    pub async fn check_subdomain_exists(&self, subdomain: &str, exclude_id: Option<&Uuid>) -> ApiResult<bool> {
        println!("[SchoolDao] Starting check_subdomain_exists for subdomain: {}", subdomain);

        let subdomain = subdomain.to_string();
        let exclude_id = exclude_id.copied();

        self.execute_with_connection(|client| async move {
            println!("[SchoolDao] Database connection acquired for subdomain check");

            let query = if let Some(exclude_id) = exclude_id {
                println!("[SchoolDao] Executing subdomain check query (excluding ID: {})", exclude_id);
                format!(
                    "SELECT COUNT(*) FROM schools WHERE subdomain = '{}' AND id != '{}' AND (is_active = true OR is_active IS NULL)",
                    subdomain.replace('\'', "''"),
                    exclude_id
                )
            } else {
                println!("[SchoolDao] Executing subdomain check query (no exclusions)");
                format!(
                    "SELECT COUNT(*) FROM schools WHERE subdomain = '{}' AND (is_active = true OR is_active IS NULL)",
                    subdomain.replace('\'', "''")
                )
            };

            let result = client.simple_query(&query).await
                .map_err(|e| {
                    println!("[SchoolDao] Subdomain check query failed: {:?}", e);
                    AppError::Database(e.to_string())
                })?;

            let count: i64 = if let Some(tokio_postgres::SimpleQueryMessage::Row(row)) = result.first() {
                row.get(0).unwrap().parse().unwrap_or(0)
            } else {
                0
            };

            println!("[SchoolDao] Subdomain check completed: subdomain={}, count={}, exists={}", subdomain, count, count > 0);
            Ok(count > 0)
        }).await
    }
}