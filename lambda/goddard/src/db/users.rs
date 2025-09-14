use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;
use crate::models::schema::{User, UserRole, CreateUserRequest};
use super::DbError;

pub async fn get_users_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    role: Option<UserRole>,
    page: i32,
    limit: i32,
) -> Result<Vec<User>, DbError> {
    let offset = (page - 1) * limit;

    let query = if let Some(role) = role {
        let role_str = match role {
            UserRole::Parent => "parent",
            UserRole::Teacher => "teacher",
            UserRole::Admin => "admin",
            UserRole::SuperAdmin => "super_admin",
        };

        format!(
            r#"
            SELECT
                u.id,
                u.school_id,
                u.email,
                u.role,
                u.metadata->>'first_name' as first_name,
                u.metadata->>'last_name' as last_name,
                u.metadata->>'phone' as phone,
                u.metadata->>'email_verified' as email_verified,
                u.metadata->>'last_login' as last_login,
                u.is_active,
                u.created_at,
                u.updated_at
            FROM users u
            WHERE u.school_id = $1
                AND u.role = '{}'
                AND u.is_active = true
            ORDER BY u.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            role_str,
            limit,
            offset
        )
    } else {
        format!(
            r#"
            SELECT
                u.id,
                u.school_id,
                u.email,
                u.role,
                u.metadata->>'first_name' as first_name,
                u.metadata->>'last_name' as last_name,
                u.metadata->>'phone' as phone,
                u.metadata->>'email_verified' as email_verified,
                u.metadata->>'last_login' as last_login,
                u.is_active,
                u.created_at,
                u.updated_at
            FROM users u
            WHERE u.school_id = $1
                AND u.is_active = true
            ORDER BY u.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            limit,
            offset
        )
    };

    let rows = sqlx::query(&query)
        .bind(school_id)
        .fetch_all(pool)
        .await?;

    let users = rows
        .into_iter()
        .map(|row| {
            let id: Uuid = row.get("id");
            let school_id_db: Option<Uuid> = row.get("school_id");
            let email: String = row.get("email");
            let role_str: String = row.get("role");
            let first_name: Option<String> = row.get("first_name");
            let last_name: Option<String> = row.get("last_name");
            let phone: Option<String> = row.get("phone");
            let email_verified: Option<String> = row.get("email_verified");
            let last_login: Option<String> = row.get("last_login");
            let is_active: Option<bool> = row.get("is_active");
            let created_at: Option<chrono::DateTime<chrono::Utc>> = row.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = row.get("updated_at");

            User {
                id,
                school_id: school_id_db.unwrap_or(school_id),
                email,
                role: match role_str.as_str() {
                    "parent" => UserRole::Parent,
                    "teacher" => UserRole::Teacher,
                    "admin" => UserRole::Admin,
                    "super_admin" => UserRole::SuperAdmin,
                    _ => UserRole::Parent,
                },
                first_name: first_name.unwrap_or_default(),
                last_name: last_name.unwrap_or_default(),
                phone,
                is_active: is_active.unwrap_or(true),
                email_verified: email_verified.map(|v| v == "true").unwrap_or(false),
                created_at: created_at.unwrap_or_else(|| chrono::Utc::now()),
                updated_at: updated_at.unwrap_or_else(|| chrono::Utc::now()),
                last_login: last_login.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
            }
        })
        .collect();

    Ok(users)
}

pub async fn get_user_by_id(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    school_id: Uuid,
) -> Result<User, DbError> {
    let row = sqlx::query!(
        r#"
        SELECT
            u.id,
            u.school_id,
            u.email,
            u.role,
            u.metadata->>'first_name' as first_name,
            u.metadata->>'last_name' as last_name,
            u.metadata->>'phone' as phone,
            u.metadata->>'email_verified' as email_verified,
            u.metadata->>'last_login' as last_login,
            u.is_active,
            u.created_at,
            u.updated_at
        FROM users u
        WHERE u.id = $1
            AND u.school_id = $2
            AND u.is_active = true
        "#,
        user_id,
        school_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    Ok(User {
        id: row.id,
        school_id: row.school_id.unwrap_or(school_id),
        email: row.email,
        role: match row.role.as_str() {
            "parent" => UserRole::Parent,
            "teacher" => UserRole::Teacher,
            "admin" => UserRole::Admin,
            "super_admin" => UserRole::SuperAdmin,
            _ => UserRole::Parent,
        },
        first_name: row.first_name.unwrap_or_default(),
        last_name: row.last_name.unwrap_or_default(),
        phone: row.phone,
        is_active: row.is_active.unwrap_or(true),
        email_verified: row.email_verified.map(|v| v == "true").unwrap_or(false),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        last_login: row.last_login.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
    })
}

pub async fn create_user(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    request: CreateUserRequest,
    created_by: Uuid,
) -> Result<User, DbError> {
    // Check if email already exists
    let existing = sqlx::query!(
        r#"
        SELECT id FROM users
        WHERE email = $1 AND is_active = true
        "#,
        request.email
    )
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(DbError::DuplicateRecord);
    }

    let role_str = match request.role {
        UserRole::Parent => "parent",
        UserRole::Teacher => "teacher",
        UserRole::Admin => "admin",
        UserRole::SuperAdmin => "super_admin",
    };

    let metadata = serde_json::json!({
        "first_name": request.first_name,
        "last_name": request.last_name,
        "phone": request.phone,
        "email_verified": false,
        "send_welcome_email": request.send_welcome_email.unwrap_or(true)
    });

    let row = sqlx::query!(
        r#"
        INSERT INTO users (
            school_id,
            email,
            role,
            metadata,
            created_by,
            updated_by
        ) VALUES ($1, $2, $3, $4, $5, $5)
        RETURNING
            id,
            school_id,
            email,
            role,
            metadata->>'first_name' as first_name,
            metadata->>'last_name' as last_name,
            metadata->>'phone' as phone,
            metadata->>'email_verified' as email_verified,
            is_active,
            created_at,
            updated_at
        "#,
        school_id,
        request.email,
        role_str,
        metadata,
        created_by
    )
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: row.id,
        school_id: row.school_id.unwrap_or(school_id),
        email: row.email,
        role: request.role,
        first_name: row.first_name.unwrap_or_default(),
        last_name: row.last_name.unwrap_or_default(),
        phone: row.phone,
        is_active: row.is_active.unwrap_or(true),
        email_verified: row.email_verified.map(|v| v == "true").unwrap_or(false),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        last_login: None,
    })
}

pub async fn update_user(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    school_id: Uuid,
    first_name: Option<String>,
    last_name: Option<String>,
    phone: Option<String>,
    updated_by: Uuid,
) -> Result<User, DbError> {
    // Get current user first
    let current = get_user_by_id(pool, user_id, school_id).await?;

    let metadata = serde_json::json!({
        "first_name": first_name.unwrap_or(current.first_name),
        "last_name": last_name.unwrap_or(current.last_name),
        "phone": phone.or(current.phone),
        "email_verified": current.email_verified,
        "last_login": current.last_login
    });

    let row = sqlx::query!(
        r#"
        UPDATE users
        SET
            metadata = $3,
            updated_by = $4,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        RETURNING
            id,
            school_id,
            email,
            role,
            metadata->>'first_name' as first_name,
            metadata->>'last_name' as last_name,
            metadata->>'phone' as phone,
            metadata->>'email_verified' as email_verified,
            metadata->>'last_login' as last_login,
            is_active,
            created_at,
            updated_at
        "#,
        user_id,
        school_id,
        metadata,
        updated_by
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    Ok(User {
        id: row.id,
        school_id: row.school_id.unwrap_or(school_id),
        email: row.email,
        role: match row.role.as_str() {
            "parent" => UserRole::Parent,
            "teacher" => UserRole::Teacher,
            "admin" => UserRole::Admin,
            "super_admin" => UserRole::SuperAdmin,
            _ => UserRole::Parent,
        },
        first_name: row.first_name.unwrap_or_default(),
        last_name: row.last_name.unwrap_or_default(),
        phone: row.phone,
        is_active: row.is_active.unwrap_or(true),
        email_verified: row.email_verified.map(|v| v == "true").unwrap_or(false),
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        last_login: row.last_login.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
    })
}

pub async fn count_users_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    role: Option<UserRole>,
) -> Result<i64, DbError> {
    let count = if let Some(role) = role {
        let role_str = match role {
            UserRole::Parent => "parent",
            UserRole::Teacher => "teacher",
            UserRole::Admin => "admin",
            UserRole::SuperAdmin => "super_admin",
        };

        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM users
            WHERE school_id = $1
                AND role = $2
                AND is_active = true
            "#,
            school_id,
            role_str
        )
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM users
            WHERE school_id = $1
                AND is_active = true
            "#,
            school_id
        )
        .fetch_one(pool)
        .await?
    };

    Ok(count)
}