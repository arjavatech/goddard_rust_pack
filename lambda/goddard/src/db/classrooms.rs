use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::schema::{Classroom, AgeGroup, ClassroomTeacher, TeacherRole, CreateClassroomRequest};
use super::DbError;

pub async fn get_classrooms_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    page: i32,
    limit: i32,
) -> Result<Vec<Classroom>, DbError> {
    let offset = (page - 1) * limit;

    let rows = sqlx::query!(
        r#"
        SELECT
            c.id,
            c.school_id,
            c.name,
            c.age_group,
            c.capacity,
            c.enrolled_count,
            c.capacity - c.enrolled_count as available_spots,
            c.is_active,
            c.created_at
        FROM classrooms c
        WHERE c.school_id = $1
            AND c.is_active = true
        ORDER BY c.name ASC
        LIMIT $2 OFFSET $3
        "#,
        school_id,
        limit as i64,
        offset as i64
    )
    .fetch_all(pool)
    .await?;

    let mut classrooms = Vec::new();

    for row in rows {
        // Get teachers for this classroom
        let teachers = get_classroom_teachers(pool, row.id).await?;

        classrooms.push(Classroom {
            id: row.id,
            school_id: row.school_id,
            name: row.name,
            age_group: parse_age_group(&row.age_group),
            capacity: row.capacity,
            enrolled_count: row.enrolled_count,
            available_spots: row.available_spots.unwrap_or(row.capacity - row.enrolled_count),
            teachers,
            is_active: row.is_active,
            created_at: row.created_at,
        });
    }

    Ok(classrooms)
}

pub async fn get_classroom_by_id(
    pool: &Pool<Postgres>,
    classroom_id: Uuid,
    school_id: Uuid,
) -> Result<Classroom, DbError> {
    let row = sqlx::query!(
        r#"
        SELECT
            c.id,
            c.school_id,
            c.name,
            c.age_group,
            c.capacity,
            c.enrolled_count,
            c.capacity - c.enrolled_count as available_spots,
            c.is_active,
            c.created_at
        FROM classrooms c
        WHERE c.id = $1
            AND c.school_id = $2
            AND c.is_active = true
        "#,
        classroom_id,
        school_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    let teachers = get_classroom_teachers(pool, row.id).await?;

    Ok(Classroom {
        id: row.id,
        school_id: row.school_id,
        name: row.name,
        age_group: parse_age_group(&row.age_group),
        capacity: row.capacity,
        enrolled_count: row.enrolled_count,
        available_spots: row.available_spots.unwrap_or(row.capacity - row.enrolled_count),
        teachers,
        is_active: row.is_active,
        created_at: row.created_at,
    })
}

pub async fn create_classroom(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    request: CreateClassroomRequest,
    created_by: Uuid,
) -> Result<Classroom, DbError> {
    let age_group_str = match request.age_group {
        AgeGroup::Infant => "infant",
        AgeGroup::Toddler => "toddler",
        AgeGroup::Preschool => "preschool",
        AgeGroup::PreK => "pre_k",
    };

    // Calculate age ranges based on age group
    let (min_age, max_age) = match request.age_group {
        AgeGroup::Infant => (0, 18),
        AgeGroup::Toddler => (19, 36),
        AgeGroup::Preschool => (37, 48),
        AgeGroup::PreK => (49, 72),
    };

    let row = sqlx::query!(
        r#"
        INSERT INTO classrooms (
            school_id,
            name,
            age_group,
            capacity,
            enrolled_count,
            min_age_months,
            max_age_months,
            created_by,
            updated_by
        ) VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $7)
        RETURNING
            id,
            school_id,
            name,
            age_group,
            capacity,
            enrolled_count,
            capacity - enrolled_count as available_spots,
            is_active,
            created_at
        "#,
        school_id,
        request.name,
        age_group_str,
        request.capacity,
        min_age,
        max_age,
        created_by
    )
    .fetch_one(pool)
    .await?;

    Ok(Classroom {
        id: row.id,
        school_id: row.school_id,
        name: row.name,
        age_group: request.age_group,
        capacity: row.capacity,
        enrolled_count: row.enrolled_count,
        available_spots: row.available_spots.unwrap_or(row.capacity),
        teachers: Vec::new(),
        is_active: row.is_active,
        created_at: row.created_at,
    })
}

pub async fn update_classroom(
    pool: &Pool<Postgres>,
    classroom_id: Uuid,
    school_id: Uuid,
    name: Option<String>,
    capacity: Option<i32>,
    updated_by: Uuid,
) -> Result<Classroom, DbError> {
    let row = sqlx::query!(
        r#"
        UPDATE classrooms
        SET
            name = COALESCE($3, name),
            capacity = COALESCE($4, capacity),
            updated_by = $5,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        RETURNING
            id,
            school_id,
            name,
            age_group,
            capacity,
            enrolled_count,
            capacity - enrolled_count as available_spots,
            is_active,
            created_at
        "#,
        classroom_id,
        school_id,
        name,
        capacity,
        updated_by
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    let teachers = get_classroom_teachers(pool, row.id).await?;

    Ok(Classroom {
        id: row.id,
        school_id: row.school_id,
        name: row.name,
        age_group: parse_age_group(&row.age_group),
        capacity: row.capacity,
        enrolled_count: row.enrolled_count,
        available_spots: row.available_spots.unwrap_or(row.capacity - row.enrolled_count),
        teachers,
        is_active: row.is_active,
        created_at: row.created_at,
    })
}

pub async fn update_classroom_enrollment_count(
    pool: &Pool<Postgres>,
    classroom_id: Uuid,
    increment: bool,
) -> Result<(), DbError> {
    let query = if increment {
        sqlx::query!(
            r#"
            UPDATE classrooms
            SET
                enrolled_count = enrolled_count + 1,
                updated_at = NOW()
            WHERE id = $1
                AND is_active = true
                AND enrolled_count < capacity
            "#,
            classroom_id
        )
    } else {
        sqlx::query!(
            r#"
            UPDATE classrooms
            SET
                enrolled_count = GREATEST(0, enrolled_count - 1),
                updated_at = NOW()
            WHERE id = $1
                AND is_active = true
            "#,
            classroom_id
        )
    };

    let result = query.execute(pool).await?;

    if result.rows_affected() == 0 {
        return Err(DbError::InvalidInput("Cannot update enrollment count".to_string()));
    }

    Ok(())
}

async fn get_classroom_teachers(
    pool: &Pool<Postgres>,
    classroom_id: Uuid,
) -> Result<Vec<ClassroomTeacher>, DbError> {
    // For now, return empty vec since we don't have teacher assignments table yet
    // This would join with a classroom_teachers table
    Ok(Vec::new())
}

pub async fn count_classrooms_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<i64, DbError> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM classrooms
        WHERE school_id = $1
            AND is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

fn parse_age_group(age_group: &Option<String>) -> AgeGroup {
    match age_group.as_deref() {
        Some("infant") => AgeGroup::Infant,
        Some("toddler") => AgeGroup::Toddler,
        Some("preschool") => AgeGroup::Preschool,
        Some("pre_k") => AgeGroup::PreK,
        _ => AgeGroup::Toddler,
    }
}