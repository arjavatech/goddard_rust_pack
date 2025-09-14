use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use crate::models::schema::School;
use super::DbError;

pub async fn get_school_by_id(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<School, DbError> {
    let row = sqlx::query!(
        r#"
        SELECT
            id,
            name,
            subdomain,
            settings,
            is_active,
            created_at,
            updated_at
        FROM schools
        WHERE id = $1 AND is_active = true
        "#,
        school_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    Ok(School {
        id: row.id,
        name: row.name,
        subdomain: row.subdomain,
        settings: row.settings
            .and_then(|s| serde_json::from_value(s).ok())
            .unwrap_or_else(|| std::collections::HashMap::new()),
        is_active: row.is_active.unwrap_or(true),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    })
}

pub async fn get_all_schools(
    pool: &Pool<Postgres>,
    page: i32,
    limit: i32,
) -> Result<Vec<School>, DbError> {
    let offset = (page - 1) * limit;

    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            name,
            subdomain,
            settings,
            is_active,
            created_at,
            updated_at
        FROM schools
        WHERE is_active = true
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        limit as i64,
        offset as i64
    )
    .fetch_all(pool)
    .await?;

    let schools = rows.into_iter().map(|row| School {
        id: row.id,
        name: row.name,
        subdomain: row.subdomain,
        settings: row.settings
            .and_then(|s| serde_json::from_value(s).ok())
            .unwrap_or_else(|| std::collections::HashMap::new()),
        is_active: row.is_active.unwrap_or(true),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    }).collect();

    Ok(schools)
}

pub async fn create_school(
    pool: &Pool<Postgres>,
    request: crate::controllers::school_controller::CreateSchoolRequest,
) -> Result<School, DbError> {
    tracing::info!("🔹 Line 90: Starting create_school function");

    // Use the default system user from database setup
    let created_by = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    tracing::info!("🔹 Line 92: Using system user ID: {}", created_by);

    // Check if subdomain already exists
    tracing::info!("🔹 Line 94: About to check if subdomain '{}' exists", request.subdomain);
    let existing = sqlx::query!(
        r#"
        SELECT id FROM schools
        WHERE subdomain = $1 AND is_active = true
        "#,
        request.subdomain
    )
    .fetch_optional(pool)
    .await?;
    tracing::info!("🔹 Line 102: Subdomain check completed. Exists: {}", existing.is_some());

    if existing.is_some() {
        tracing::info!("🔹 Line 105: Subdomain already exists, returning DuplicateRecord error");
        return Err(DbError::DuplicateRecord);
    }

    tracing::info!("🔹 Line 108: About to INSERT into schools table");
    tracing::info!("🔹 Values: name='{}', subdomain='{}', created_by='{}'", request.name, request.subdomain, created_by);
    let row = sqlx::query!(
        r#"
        INSERT INTO schools (
            name,
            subdomain,
            settings,
            created_by,
            updated_by
        ) VALUES ($1, $2, $3, $4, $4)
        RETURNING
            id,
            name,
            subdomain,
            settings,
            is_active,
            created_at,
            updated_at
        "#,
        request.name,
        request.subdomain,
        serde_json::to_value(&request.settings.unwrap_or_else(|| std::collections::HashMap::new())).unwrap(),
        created_by
    )
    .fetch_one(pool)
    .await?;
    tracing::info!("🔹 Line 132: INSERT completed successfully");

    tracing::info!("🔹 Line 134: Building School response object");
    Ok(School {
        id: row.id,
        name: row.name,
        subdomain: row.subdomain,
        settings: row.settings
            .and_then(|s| serde_json::from_value(s).ok())
            .unwrap_or_else(|| std::collections::HashMap::new()),
        is_active: row.is_active.unwrap_or(true),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    })
}

// Temporarily commented out to avoid type issues
// pub async fn update_school(...) { ... }

// Temporarily commented out
// pub async fn delete_school(...) { ... }

pub async fn count_schools(pool: &Pool<Postgres>) -> Result<i64, DbError> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM schools
        WHERE is_active = true
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

// Alias for compatibility with controller
pub async fn count_all_schools(pool: &Pool<Postgres>) -> Result<i64, DbError> {
    count_schools(pool).await
}

// Get first school (for single school endpoint)
pub async fn get_first_school(pool: &Pool<Postgres>) -> Result<School, DbError> {
    let row = sqlx::query!(
        r#"
        SELECT
            id,
            name,
            subdomain,
            settings,
            is_active,
            created_at,
            updated_at
        FROM schools
        WHERE is_active = true
        ORDER BY created_at ASC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    Ok(School {
        id: row.id,
        name: row.name,
        subdomain: row.subdomain,
        settings: row.settings
            .and_then(|s| serde_json::from_value(s).ok())
            .unwrap_or_else(|| std::collections::HashMap::new()),
        is_active: row.is_active.unwrap_or(true),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    })
}