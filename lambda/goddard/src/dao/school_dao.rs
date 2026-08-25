use deadpool_postgres::{Pool, Client};
use tokio_postgres::Row;
use uuid::Uuid;
use crate::{
    error::{AppError, ApiResult},
    models::school::{School, CreateSchoolRequest, UpdateSchoolRequest, RequestSettingOption, RequestSettingsOperation, SchoolRequestSettingsResponse},
};
use std::time::Duration;

#[derive(Clone)]
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
            timezone: row.get("timezone"),
            settings: row.get("settings"),
            request_categories: row.get("request_categories"),
            location: row.get("location"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn create_school(&self, request: &CreateSchoolRequest) -> ApiResult<School> {
        println!("[SchoolDao] Starting create_school - getting database connection");

        let name = request.name.clone();
        let subdomain = request.subdomain.clone();
        let timezone = request.timezone.clone().unwrap_or_else(|| "EST".to_string());
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
                INSERT INTO schools (id, name, subdomain, timezone, settings, is_active, created_at)
                VALUES (gen_random_uuid(), '{}', '{}', '{}', {}, true, NOW())
                RETURNING id, name, subdomain, timezone, settings, NULL::jsonb AS request_categories, NULL::jsonb AS location, is_active, created_at, updated_at
                "#,
                name.replace('\'', "''"),
                subdomain.replace('\'', "''"),
                timezone.replace('\'', "''"),
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
                        timezone: row.get(3).unwrap_or("EST").to_string(),
                        settings: row.get(4).and_then(|s| serde_json::from_str(s).ok()),
                        request_categories: row.get(5).and_then(|s| serde_json::from_str(s).ok()),
                        location: row.get(6).and_then(|s| serde_json::from_str(s).ok()),
                        is_active: row.get(7).and_then(|s| s.parse().ok()),
                        created_at: row.get(8).and_then(|s| s.parse().ok()),
                        updated_at: row.get(9).and_then(|s| s.parse().ok()),
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
                SELECT id, name, subdomain, timezone, settings, request_categories, location, is_active, created_at, updated_at
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
                        timezone: row.get(3).unwrap_or("EST").to_string(),
                        settings: row.get(4).and_then(|s| serde_json::from_str(s).ok()),
                        request_categories: row.get(5).and_then(|s| serde_json::from_str(s).ok()),
                        location: row.get(6).and_then(|s| serde_json::from_str(s).ok()),
                        is_active: row.get(7).and_then(|s| s.parse().ok()),
                        created_at: row.get(8).and_then(|s| s.parse().ok()),
                        updated_at: row.get(9).and_then(|s| s.parse().ok()),
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
                SELECT id, name, subdomain, timezone, settings, request_categories, location, is_active, created_at, updated_at
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
                        timezone: row.get(3).unwrap_or("EST").to_string(),
                        settings: row.get(4).and_then(|s| serde_json::from_str(s).ok()),
                        request_categories: row.get(5).and_then(|s| serde_json::from_str(s).ok()),
                        location: row.get(6).and_then(|s| serde_json::from_str(s).ok()),
                        is_active: row.get(7).and_then(|s| s.parse().ok()),
                        created_at: row.get(8).and_then(|s| s.parse().ok()),
                        updated_at: row.get(9).and_then(|s| s.parse().ok()),
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
                timezone = '{}',
                settings = jsonb_set(COALESCE(settings, '{{}}'::jsonb), '{{timezone}}', to_jsonb('{}'::text), true),
                updated_at = NOW()
            WHERE id = '{}' AND (is_active = true OR is_active IS NULL)
            RETURNING id, name, subdomain, timezone, settings, request_categories, location, is_active, created_at, updated_at
            "#,
            request.name.replace('\'', "''"),
            request.subdomain.replace('\'', "''"),
            request.timezone.clone().unwrap_or_else(|| "EST".to_string()).replace('\'', "''"),
            request.timezone.clone().unwrap_or_else(|| "EST".to_string()).replace('\'', "''"),
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
                    timezone: row.get(3).unwrap_or("EST").to_string(),
                    settings: row.get(4).and_then(|s| serde_json::from_str(s).ok()),
                    request_categories: row.get(5).and_then(|s| serde_json::from_str(s).ok()),
                    location: row.get(6).and_then(|s| serde_json::from_str(s).ok()),
                    is_active: row.get(7).and_then(|s| s.parse().ok()),
                    created_at: row.get(8).and_then(|s| s.parse().ok()),
                    updated_at: row.get(9).and_then(|s| s.parse().ok()),
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

    /// Get school name by ID for email personalization
    pub async fn get_school_name(&self, school_id: &Uuid) -> ApiResult<String> {
        tracing::info!("🔍 [SchoolDao] Fetching school name for school_id: {}", school_id);

        self.execute_with_connection(|client| async move {
            let query = "SELECT name FROM schools WHERE id = $1";
            let row = client
                .query_one(query, &[school_id])
                .await
                .map_err(|e| {
                    tracing::error!("❌ [SchoolDao] Failed to fetch school name for {}: {}", school_id, e);
                    AppError::Database(format!("Failed to fetch school name: {}", e))
                })?;

            let name: String = row.get("name");
            tracing::info!("✅ [SchoolDao] Successfully fetched school name: '{}' for school_id: {}", name, school_id);
            Ok(name)
        }).await
    }

    pub async fn get_request_settings(&self, school_id: Uuid) -> ApiResult<SchoolRequestSettingsResponse> {
        let client = self.get_connection().await?;
        let row = client.query_opt(
            "SELECT request_categories, location FROM schools WHERE id = $1 AND (is_active = true OR is_active IS NULL)",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get request settings: {}", e)))?
            .ok_or_else(|| AppError::NotFound("School not found".to_string()))?;

        Ok(Self::request_settings_from_values(
            school_id,
            row.get("request_categories"),
            row.get("location"),
        ))
    }

    pub async fn update_request_settings(
        &self,
        school_id: Uuid,
        operations: &[RequestSettingsOperation],
    ) -> ApiResult<SchoolRequestSettingsResponse> {
        if operations.is_empty() {
            return Err(AppError::Validation("At least one settings operation is required".to_string()));
        }

        let mut client = self.get_connection().await?;
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start request settings transaction: {}", e)))?;
        let row = transaction.query_opt(
            "SELECT request_categories, location FROM schools WHERE id = $1 AND (is_active = true OR is_active IS NULL) FOR UPDATE",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to lock school request settings: {}", e)))?
            .ok_or_else(|| AppError::NotFound("School not found".to_string()))?;

        let mut categories = Self::options_from_value(row.get("request_categories"));
        let mut locations = Self::options_from_value(row.get("location"));

        for operation in operations {
            let options = match operation.setting.as_str() {
                "request_categories" => &mut categories,
                "location" => &mut locations,
                _ => return Err(AppError::Validation("setting must be request_categories or location".to_string())),
            };

            match operation.operation.as_str() {
                "add" => {
                    let label = Self::validated_label(operation.label.as_deref())?;
                    Self::ensure_unique_label(options, &label, None)?;
                    options.push(RequestSettingOption { id: Uuid::new_v4(), label });
                }
                "update" => {
                    let option_id = operation.option_id.ok_or_else(|| AppError::Validation("optionId is required for update".to_string()))?;
                    let label = Self::validated_label(operation.label.as_deref())?;
                    let index = options.iter().position(|item| item.id == option_id)
                        .ok_or_else(|| AppError::NotFound("Request setting option not found".to_string()))?;
                    let old_label = options[index].label.clone();
                    Self::ensure_unique_label(options, &label, Some(option_id))?;
                    options[index].label = label.clone();
                    let column = if operation.setting == "request_categories" { "category" } else { "location" };
                    let query = format!("UPDATE requests SET {} = $3 WHERE school_id = $1 AND {} = $2", column, column);
                    transaction.execute(&query, &[&school_id, &old_label, &label]).await
                        .map_err(|e| AppError::Database(format!("Failed to rename request setting values: {}", e)))?;
                }
                "delete" => {
                    if operation.label.is_some() {
                        return Err(AppError::Validation("label must be empty for delete".to_string()));
                    }
                    let option_id = operation.option_id.ok_or_else(|| AppError::Validation("optionId is required for delete".to_string()))?;
                    let index = options.iter().position(|item| item.id == option_id)
                        .ok_or_else(|| AppError::NotFound("Request setting option not found".to_string()))?;
                    let old_label = options.remove(index).label;
                    let column = if operation.setting == "request_categories" { "category" } else { "location" };
                    let query = format!("UPDATE requests SET {} = NULL WHERE school_id = $1 AND {} = $2", column, column);
                    transaction.execute(&query, &[&school_id, &old_label]).await
                        .map_err(|e| AppError::Database(format!("Failed to clear deleted request setting values: {}", e)))?;
                }
                _ => return Err(AppError::Validation("operation must be add, update, or delete".to_string())),
            }
        }

        let category_value = serde_json::to_value(&categories)
            .map_err(|e| AppError::Internal(format!("Failed to serialize categories: {}", e)))?;
        let location_value = serde_json::to_value(&locations)
            .map_err(|e| AppError::Internal(format!("Failed to serialize locations: {}", e)))?;
        transaction.execute(
            "UPDATE schools SET request_categories = $2, location = $3, updated_at = NOW() WHERE id = $1",
            &[&school_id, &category_value, &location_value],
        ).await.map_err(|e| AppError::Database(format!("Failed to save request settings: {}", e)))?;
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit request settings: {}", e)))?;

        Ok(Self::request_settings_from_options(school_id, categories, locations))
    }

    fn request_settings_from_values(
        school_id: Uuid,
        categories: Option<serde_json::Value>,
        locations: Option<serde_json::Value>,
    ) -> SchoolRequestSettingsResponse {
        Self::request_settings_from_options(school_id, Self::options_from_value(categories), Self::options_from_value(locations))
    }

    fn request_settings_from_options(
        school_id: Uuid,
        request_categories: Vec<RequestSettingOption>,
        location: Vec<RequestSettingOption>,
    ) -> SchoolRequestSettingsResponse {
        SchoolRequestSettingsResponse {
            school_id,
            request_categories,
            location,
            csv_fields: vec!["category".to_string(), "location".to_string()],
        }
    }

    fn options_from_value(value: Option<serde_json::Value>) -> Vec<RequestSettingOption> {
        value.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    fn validated_label(value: Option<&str>) -> ApiResult<String> {
        let label = value.unwrap_or_default().trim();
        if label.is_empty() {
            return Err(AppError::Validation("label is required".to_string()));
        }
        if label.len() > 255 {
            return Err(AppError::Validation("label must be at most 255 characters".to_string()));
        }
        Ok(label.to_string())
    }

    fn ensure_unique_label(options: &[RequestSettingOption], label: &str, exclude_id: Option<Uuid>) -> ApiResult<()> {
        if options.iter().any(|item| Some(item.id) != exclude_id && item.label.eq_ignore_ascii_case(label)) {
            return Err(AppError::Conflict("A setting with this label already exists".to_string()));
        }
        Ok(())
    }
}
