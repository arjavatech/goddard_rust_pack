use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::NaiveDate;
use crate::models::schema::{
    Enrollment, EnrollmentWorkflowStatus, AdminApprovalStatus, EnrollmentProgress,
    CreateEnrollmentRequest, UpdateEnrollmentRequest, ApproveEnrollmentRequest,
    RejectEnrollmentRequest
};
use super::DbError;

pub async fn get_enrollments_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    status: Option<EnrollmentWorkflowStatus>,
    page: i32,
    limit: i32,
) -> Result<Vec<Enrollment>, DbError> {
    let offset = (page - 1) * limit;

    let rows = if let Some(status) = status {
        let status_str = match status {
            EnrollmentWorkflowStatus::Pending => "pending",
            EnrollmentWorkflowStatus::InProgress => "in_progress",
            EnrollmentWorkflowStatus::UnderReview => "under_review",
            EnrollmentWorkflowStatus::Approved => "approved",
            EnrollmentWorkflowStatus::Rejected => "rejected",
            EnrollmentWorkflowStatus::NeedsRevision => "needs_revision",
        };

        sqlx::query!(
            r#"
            SELECT
                e.id,
                e.child_id,
                e.school_id,
                e.classroom_id,
                e.status,
                e.admin_approval_status,
                e.progress,
                e.start_date,
                e.created_at,
                e.updated_at
            FROM enrollments e
            WHERE e.school_id = $1
                AND e.status = $2
                AND e.is_active = true
            ORDER BY e.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            school_id,
            status_str,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query!(
            r#"
            SELECT
                e.id,
                e.child_id,
                e.school_id,
                e.classroom_id,
                e.status,
                e.admin_approval_status,
                e.progress,
                e.start_date,
                e.created_at,
                e.updated_at
            FROM enrollments e
            WHERE e.school_id = $1
                AND e.is_active = true
            ORDER BY e.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            school_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?
    };

    let enrollments = rows
        .into_iter()
        .map(|row| {
            let progress: Option<EnrollmentProgress> = row.progress
                .and_then(|json| serde_json::from_value(json).ok());

            Enrollment {
                id: row.id,
                child_id: row.child_id,
                school_id: row.school_id,
                classroom_id: row.classroom_id,
                status: parse_enrollment_status(&row.status),
                admin_approval_status: parse_admin_status(&row.admin_approval_status),
                progress,
                start_date: row.start_date,
                child: None,
                parent: None,
                classroom: None,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }
        })
        .collect();

    Ok(enrollments)
}

pub async fn get_enrollment_by_id(
    pool: &Pool<Postgres>,
    enrollment_id: Uuid,
    school_id: Uuid,
) -> Result<Enrollment, DbError> {
    let row = sqlx::query!(
        r#"
        SELECT
            e.id,
            e.child_id,
            e.school_id,
            e.classroom_id,
            e.status,
            e.admin_approval_status,
            e.progress,
            e.start_date,
            e.created_at,
            e.updated_at
        FROM enrollments e
        WHERE e.id = $1
            AND e.school_id = $2
            AND e.is_active = true
        "#,
        enrollment_id,
        school_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    let progress: Option<EnrollmentProgress> = row.progress
        .and_then(|json| serde_json::from_value(json).ok());

    Ok(Enrollment {
        id: row.id,
        child_id: row.child_id,
        school_id: row.school_id,
        classroom_id: row.classroom_id,
        status: parse_enrollment_status(&row.status),
        admin_approval_status: parse_admin_status(&row.admin_approval_status),
        progress,
        start_date: row.start_date,
        child: None,
        parent: None,
        classroom: None,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn create_enrollment(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    request: CreateEnrollmentRequest,
    created_by: Uuid,
) -> Result<Enrollment, DbError> {
    // Check if classroom has available spots
    let classroom_check = sqlx::query!(
        r#"
        SELECT capacity, enrolled_count
        FROM classrooms
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        request.classroom_id,
        school_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    if classroom_check.enrolled_count >= classroom_check.capacity {
        return Err(DbError::InvalidInput("Classroom is full".to_string()));
    }

    // Check if child already has active enrollment
    let existing = sqlx::query!(
        r#"
        SELECT id FROM enrollments
        WHERE child_id = $1
            AND school_id = $2
            AND status NOT IN ('rejected', 'withdrawn')
            AND is_active = true
        "#,
        request.child_id,
        school_id
    )
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(DbError::DuplicateRecord);
    }

    // Generate enrollment number
    let enrollment_number = format!("ENR-{}", Uuid::new_v4().to_string()[0..8].to_uppercase());
    let academic_year = format!("{}-{}",
        chrono::Utc::now().year(),
        chrono::Utc::now().year() + 1
    );

    let initial_progress = EnrollmentProgress {
        total_forms: 0,
        completed_forms: 0,
        pending_forms: 0,
        completion_percentage: 0.0,
    };

    let row = sqlx::query!(
        r#"
        INSERT INTO enrollments (
            child_id,
            school_id,
            classroom_id,
            enrollment_number,
            academic_year,
            status,
            admin_approval_status,
            progress,
            start_date,
            created_by,
            updated_by
        ) VALUES ($1, $2, $3, $4, $5, 'pending', 'pending', $6, $7, $8, $8)
        RETURNING
            id,
            child_id,
            school_id,
            classroom_id,
            status,
            admin_approval_status,
            progress,
            start_date,
            created_at,
            updated_at
        "#,
        request.child_id,
        school_id,
        request.classroom_id,
        enrollment_number,
        academic_year,
        serde_json::to_value(&initial_progress).unwrap(),
        request.start_date,
        created_by
    )
    .fetch_one(pool)
    .await?;

    // Update classroom enrollment count
    super::classrooms::update_classroom_enrollment_count(pool, request.classroom_id, true).await?;

    Ok(Enrollment {
        id: row.id,
        child_id: row.child_id,
        school_id: row.school_id,
        classroom_id: row.classroom_id,
        status: parse_enrollment_status(&row.status),
        admin_approval_status: parse_admin_status(&row.admin_approval_status),
        progress: Some(initial_progress),
        start_date: row.start_date,
        child: None,
        parent: None,
        classroom: None,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn update_enrollment(
    pool: &Pool<Postgres>,
    enrollment_id: Uuid,
    school_id: Uuid,
    request: UpdateEnrollmentRequest,
    updated_by: Uuid,
) -> Result<Enrollment, DbError> {
    let status_str = request.status.as_ref().map(|s| match s {
        EnrollmentWorkflowStatus::Pending => "pending",
        EnrollmentWorkflowStatus::InProgress => "in_progress",
        EnrollmentWorkflowStatus::UnderReview => "under_review",
        EnrollmentWorkflowStatus::Approved => "approved",
        EnrollmentWorkflowStatus::Rejected => "rejected",
        EnrollmentWorkflowStatus::NeedsRevision => "needs_revision",
    });

    let row = sqlx::query!(
        r#"
        UPDATE enrollments
        SET
            status = COALESCE($3, status),
            updated_by = $4,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        RETURNING
            id,
            child_id,
            school_id,
            classroom_id,
            status,
            admin_approval_status,
            progress,
            start_date,
            created_at,
            updated_at
        "#,
        enrollment_id,
        school_id,
        status_str,
        updated_by
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    let progress: Option<EnrollmentProgress> = row.progress
        .and_then(|json| serde_json::from_value(json).ok());

    Ok(Enrollment {
        id: row.id,
        child_id: row.child_id,
        school_id: row.school_id,
        classroom_id: row.classroom_id,
        status: parse_enrollment_status(&row.status),
        admin_approval_status: parse_admin_status(&row.admin_approval_status),
        progress,
        start_date: row.start_date,
        child: None,
        parent: None,
        classroom: None,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn approve_enrollment(
    pool: &Pool<Postgres>,
    enrollment_id: Uuid,
    school_id: Uuid,
    request: ApproveEnrollmentRequest,
    approved_by: Uuid,
) -> Result<(), DbError> {
    let result = sqlx::query!(
        r#"
        UPDATE enrollments
        SET
            status = 'approved',
            admin_approval_status = 'approved',
            approved_at = NOW(),
            approved_by = $3,
            approval_notes = $4,
            forms_locked_at = CASE WHEN $5 THEN NOW() ELSE forms_locked_at END,
            updated_by = $3,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        enrollment_id,
        school_id,
        approved_by,
        request.approval_notes,
        request.lock_forms.unwrap_or(true)
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }

    // Create approval audit record
    sqlx::query!(
        r#"
        INSERT INTO enrollment_approval_audit (
            school_id,
            enrollment_id,
            admin_id,
            action,
            new_status,
            notes
        ) VALUES ($1, $2, $3, 'approve', 'approved', $4)
        "#,
        school_id,
        enrollment_id,
        approved_by,
        request.approval_notes
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn reject_enrollment(
    pool: &Pool<Postgres>,
    enrollment_id: Uuid,
    school_id: Uuid,
    request: RejectEnrollmentRequest,
    rejected_by: Uuid,
) -> Result<(), DbError> {
    // Get current enrollment to update classroom count if needed
    let enrollment = get_enrollment_by_id(pool, enrollment_id, school_id).await?;

    let result = sqlx::query!(
        r#"
        UPDATE enrollments
        SET
            status = 'rejected',
            admin_approval_status = 'rejected',
            approval_notes = $3,
            updated_by = $4,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        enrollment_id,
        school_id,
        request.rejection_notes,
        rejected_by
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }

    // Update classroom enrollment count if needed
    if let Some(classroom_id) = enrollment.classroom_id {
        super::classrooms::update_classroom_enrollment_count(pool, classroom_id, false).await?;
    }

    // Create rejection audit record
    sqlx::query!(
        r#"
        INSERT INTO enrollment_approval_audit (
            school_id,
            enrollment_id,
            admin_id,
            action,
            new_status,
            notes
        ) VALUES ($1, $2, $3, 'reject', 'rejected', $4)
        "#,
        school_id,
        enrollment_id,
        rejected_by,
        request.rejection_notes
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn count_enrollments_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    status: Option<EnrollmentWorkflowStatus>,
) -> Result<i64, DbError> {
    let count = if let Some(status) = status {
        let status_str = match status {
            EnrollmentWorkflowStatus::Pending => "pending",
            EnrollmentWorkflowStatus::InProgress => "in_progress",
            EnrollmentWorkflowStatus::UnderReview => "under_review",
            EnrollmentWorkflowStatus::Approved => "approved",
            EnrollmentWorkflowStatus::Rejected => "rejected",
            EnrollmentWorkflowStatus::NeedsRevision => "needs_revision",
        };

        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM enrollments
            WHERE school_id = $1
                AND status = $2
                AND is_active = true
            "#,
            school_id,
            status_str
        )
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM enrollments
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

fn parse_enrollment_status(status: &str) -> EnrollmentWorkflowStatus {
    match status {
        "pending" => EnrollmentWorkflowStatus::Pending,
        "in_progress" => EnrollmentWorkflowStatus::InProgress,
        "under_review" => EnrollmentWorkflowStatus::UnderReview,
        "approved" => EnrollmentWorkflowStatus::Approved,
        "rejected" => EnrollmentWorkflowStatus::Rejected,
        "needs_revision" => EnrollmentWorkflowStatus::NeedsRevision,
        _ => EnrollmentWorkflowStatus::Pending,
    }
}

fn parse_admin_status(status: &Option<String>) -> AdminApprovalStatus {
    match status.as_deref() {
        Some("pending") => AdminApprovalStatus::Pending,
        Some("approved") => AdminApprovalStatus::Approved,
        Some("rejected") => AdminApprovalStatus::Rejected,
        Some("needs_revision") => AdminApprovalStatus::NeedsRevision,
        _ => AdminApprovalStatus::Pending,
    }
}