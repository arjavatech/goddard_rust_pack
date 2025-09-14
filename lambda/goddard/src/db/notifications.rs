use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::schema::{AdditionalEmail, CreateAdditionalEmailRequest};
use super::DbError;

pub async fn get_additional_emails_by_parent(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    parent_id: Uuid,
) -> Result<Vec<AdditionalEmail>, DbError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            school_id,
            parent_id,
            email_address,
            email_type,
            is_verified,
            created_at,
            updated_at
        FROM parent_additional_emails
        WHERE school_id = $1
            AND parent_id = $2
            AND is_active = true
        ORDER BY created_at ASC
        "#,
        school_id,
        parent_id
    )
    .fetch_all(pool)
    .await?;

    let emails = rows
        .into_iter()
        .map(|row| AdditionalEmail {
            id: row.id,
            school_id: row.school_id,
            parent_id: row.parent_id,
            email: row.email_address,
            email_type: row.email_type,
            is_verified: row.is_verified,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(emails)
}

pub async fn add_additional_email(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    parent_id: Uuid,
    request: CreateAdditionalEmailRequest,
) -> Result<AdditionalEmail, DbError> {
    // Check if email already exists for this parent
    let existing = sqlx::query!(
        r#"
        SELECT id FROM parent_additional_emails
        WHERE parent_id = $1
            AND email_address = $2
            AND is_active = true
        "#,
        parent_id,
        request.email
    )
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(DbError::DuplicateRecord);
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO parent_additional_emails (
            school_id,
            parent_id,
            email_address,
            email_type
        ) VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            school_id,
            parent_id,
            email_address,
            email_type,
            is_verified,
            created_at,
            updated_at
        "#,
        school_id,
        parent_id,
        request.email,
        request.email_type
    )
    .fetch_one(pool)
    .await?;

    Ok(AdditionalEmail {
        id: row.id,
        school_id: row.school_id,
        parent_id: row.parent_id,
        email: row.email_address,
        email_type: row.email_type,
        is_verified: row.is_verified,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn verify_additional_email(
    pool: &Pool<Postgres>,
    email_id: Uuid,
    school_id: Uuid,
) -> Result<(), DbError> {
    let result = sqlx::query!(
        r#"
        UPDATE parent_additional_emails
        SET
            is_verified = true,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        email_id,
        school_id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }

    Ok(())
}