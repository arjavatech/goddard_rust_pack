use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::schema::{Child, AgeGroup, EnrollmentStatus, CreateChildRequest, MedicalInfo};
use super::DbError;

pub async fn get_children_by_parent(
    pool: &Pool<Postgres>,
    parent_id: Uuid,
    school_id: Uuid,
    page: i32,
    limit: i32,
) -> Result<Vec<Child>, DbError> {
    let offset = (page - 1) * limit;

    let rows = sqlx::query!(
        r#"
        SELECT
            c.id,
            c.parent_id,
            c.school_id,
            c.first_name,
            c.last_name,
            c.birth_date,
            c.medical_info,
            c.is_active,
            c.created_at,
            c.updated_at,
            CASE
                WHEN e.status = 'approved' THEN 'enrolled'
                WHEN e.status = 'pending' OR e.status = 'under_review' THEN 'pending'
                WHEN e.status = 'withdrawn' THEN 'withdrawn'
                ELSE 'pending'
            END as enrollment_status
        FROM children c
        LEFT JOIN enrollments e ON c.id = e.child_id AND e.is_active = true
        WHERE c.parent_id = $1
            AND c.school_id = $2
            AND c.is_active = true
        ORDER BY c.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
        parent_id,
        school_id,
        limit as i64,
        offset as i64
    )
    .fetch_all(pool)
    .await?;

    let children = rows
        .into_iter()
        .map(|row| {
            let age_group = determine_age_group(&row.birth_date);
            let medical_info: Option<MedicalInfo> = row.medical_info
                .and_then(|json| serde_json::from_value(json).ok());

            Child {
                id: row.id,
                parent_id: row.parent_id,
                school_id: row.school_id,
                first_name: row.first_name,
                last_name: row.last_name,
                birth_date: row.birth_date,
                age_group,
                medical_info,
                enrollment_status: match row.enrollment_status.as_deref() {
                    Some("enrolled") => EnrollmentStatus::Enrolled,
                    Some("pending") => EnrollmentStatus::Pending,
                    Some("withdrawn") => EnrollmentStatus::Withdrawn,
                    Some("graduated") => EnrollmentStatus::Graduated,
                    _ => EnrollmentStatus::Pending,
                },
                created_at: row.created_at,
                updated_at: row.updated_at,
            }
        })
        .collect();

    Ok(children)
}

pub async fn get_all_children_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    page: i32,
    limit: i32,
) -> Result<Vec<Child>, DbError> {
    let offset = (page - 1) * limit;

    let rows = sqlx::query!(
        r#"
        SELECT
            c.id,
            c.parent_id,
            c.school_id,
            c.first_name,
            c.last_name,
            c.birth_date,
            c.medical_info,
            c.is_active,
            c.created_at,
            c.updated_at,
            CASE
                WHEN e.status = 'approved' THEN 'enrolled'
                WHEN e.status = 'pending' OR e.status = 'under_review' THEN 'pending'
                WHEN e.status = 'withdrawn' THEN 'withdrawn'
                ELSE 'pending'
            END as enrollment_status
        FROM children c
        LEFT JOIN enrollments e ON c.id = e.child_id AND e.is_active = true
        WHERE c.school_id = $1
            AND c.is_active = true
        ORDER BY c.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        school_id,
        limit as i64,
        offset as i64
    )
    .fetch_all(pool)
    .await?;

    let children = rows
        .into_iter()
        .map(|row| {
            let age_group = determine_age_group(&row.birth_date);
            let medical_info: Option<MedicalInfo> = row.medical_info
                .and_then(|json| serde_json::from_value(json).ok());

            Child {
                id: row.id,
                parent_id: row.parent_id,
                school_id: row.school_id,
                first_name: row.first_name,
                last_name: row.last_name,
                birth_date: row.birth_date,
                age_group,
                medical_info,
                enrollment_status: match row.enrollment_status.as_deref() {
                    Some("enrolled") => EnrollmentStatus::Enrolled,
                    Some("pending") => EnrollmentStatus::Pending,
                    Some("withdrawn") => EnrollmentStatus::Withdrawn,
                    Some("graduated") => EnrollmentStatus::Graduated,
                    _ => EnrollmentStatus::Pending,
                },
                created_at: row.created_at,
                updated_at: row.updated_at,
            }
        })
        .collect();

    Ok(children)
}

pub async fn create_child(
    pool: &Pool<Postgres>,
    parent_id: Uuid,
    school_id: Uuid,
    request: CreateChildRequest,
    created_by: Uuid,
) -> Result<Child, DbError> {
    let medical_info_json = request.medical_info
        .map(|info| serde_json::to_value(info).unwrap_or_else(|_| serde_json::json!({})))
        .unwrap_or_else(|| serde_json::json!({}));

    let row = sqlx::query!(
        r#"
        INSERT INTO children (
            parent_id,
            school_id,
            first_name,
            last_name,
            birth_date,
            medical_info,
            created_by,
            updated_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        RETURNING
            id,
            parent_id,
            school_id,
            first_name,
            last_name,
            birth_date,
            medical_info,
            is_active,
            created_at,
            updated_at
        "#,
        parent_id,
        school_id,
        request.first_name,
        request.last_name,
        request.birth_date,
        medical_info_json,
        created_by
    )
    .fetch_one(pool)
    .await?;

    let age_group = determine_age_group(&row.birth_date);
    let medical_info: Option<MedicalInfo> = row.medical_info
        .and_then(|json| serde_json::from_value(json).ok());

    Ok(Child {
        id: row.id,
        parent_id: row.parent_id,
        school_id: row.school_id,
        first_name: row.first_name,
        last_name: row.last_name,
        birth_date: row.birth_date,
        age_group,
        medical_info,
        enrollment_status: EnrollmentStatus::Pending,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn update_child(
    pool: &Pool<Postgres>,
    child_id: Uuid,
    parent_id: Uuid,
    school_id: Uuid,
    first_name: Option<String>,
    last_name: Option<String>,
    medical_info: Option<MedicalInfo>,
    updated_by: Uuid,
) -> Result<Child, DbError> {
    let medical_info_json = medical_info
        .map(|info| serde_json::to_value(info).unwrap_or_else(|_| serde_json::json!({})));

    let row = sqlx::query!(
        r#"
        UPDATE children
        SET
            first_name = COALESCE($4, first_name),
            last_name = COALESCE($5, last_name),
            medical_info = COALESCE($6, medical_info),
            updated_by = $7,
            updated_at = NOW()
        WHERE id = $1
            AND parent_id = $2
            AND school_id = $3
            AND is_active = true
        RETURNING
            id,
            parent_id,
            school_id,
            first_name,
            last_name,
            birth_date,
            medical_info,
            is_active,
            created_at,
            updated_at
        "#,
        child_id,
        parent_id,
        school_id,
        first_name,
        last_name,
        medical_info_json,
        updated_by
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    let age_group = determine_age_group(&row.birth_date);
    let medical_info: Option<MedicalInfo> = row.medical_info
        .and_then(|json| serde_json::from_value(json).ok());

    Ok(Child {
        id: row.id,
        parent_id: row.parent_id,
        school_id: row.school_id,
        first_name: row.first_name,
        last_name: row.last_name,
        birth_date: row.birth_date,
        age_group,
        medical_info,
        enrollment_status: EnrollmentStatus::Pending,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn count_children_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<i64, DbError> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM children
        WHERE school_id = $1
            AND is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn count_children_by_parent(
    pool: &Pool<Postgres>,
    parent_id: Uuid,
    school_id: Uuid,
) -> Result<i64, DbError> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM children
        WHERE parent_id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        parent_id,
        school_id
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

fn determine_age_group(birth_date: &chrono::NaiveDate) -> AgeGroup {
    let today = chrono::Utc::now().date_naive();
    let age_days = today.signed_duration_since(*birth_date).num_days();
    let age_months = age_days / 30; // Approximate

    match age_months {
        0..=18 => AgeGroup::Infant,
        19..=36 => AgeGroup::Toddler,
        37..=48 => AgeGroup::Preschool,
        _ => AgeGroup::PreK,
    }
}