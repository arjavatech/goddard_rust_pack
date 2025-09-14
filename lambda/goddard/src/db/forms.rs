use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::schema::{
    FormTemplate, FormType, FormStatus, FormSubmission, SubmissionStatus,
    CreateFormTemplateRequest, FilloutWebhookRequest, EnrollmentProgress
};
use super::DbError;

pub async fn get_form_templates_by_school(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    form_type: Option<FormType>,
    page: i32,
    limit: i32,
) -> Result<Vec<FormTemplate>, DbError> {
    let offset = (page - 1) * limit;

    let rows = if let Some(form_type) = form_type {
        let type_str = match form_type {
            FormType::Admission => "admission",
            FormType::Medical => "medical",
            FormType::Emergency => "emergency",
            FormType::Authorization => "authorization",
            FormType::Handbook => "handbook",
            FormType::Agreement => "agreement",
        };

        sqlx::query!(
            r#"
            SELECT
                id,
                school_id,
                form_name,
                form_type,
                fillout_form_id,
                fillout_form_url,
                status,
                is_required,
                display_order,
                created_at,
                updated_at
            FROM form_templates
            WHERE school_id = $1
                AND form_type = $2
                AND is_active = true
            ORDER BY display_order ASC, form_name ASC
            LIMIT $3 OFFSET $4
            "#,
            school_id,
            type_str,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query!(
            r#"
            SELECT
                id,
                school_id,
                form_name,
                form_type,
                fillout_form_id,
                fillout_form_url,
                status,
                is_required,
                display_order,
                created_at,
                updated_at
            FROM form_templates
            WHERE school_id = $1
                AND is_active = true
            ORDER BY display_order ASC, form_name ASC
            LIMIT $2 OFFSET $3
            "#,
            school_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?
    };

    let templates = rows
        .into_iter()
        .map(|row| FormTemplate {
            id: row.id,
            school_id: row.school_id,
            form_name: row.form_name,
            form_description: None, // We don't have this field in DB
            form_type: parse_form_type(&row.form_type),
            fillout_form_id: row.fillout_form_id.unwrap_or_default(),
            fillout_form_url: row.fillout_form_url.unwrap_or_default(),
            status: parse_form_status(&row.status),
            is_required: row.is_required,
            display_order: row.display_order,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(templates)
}

pub async fn create_form_template(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    request: CreateFormTemplateRequest,
    created_by: Uuid,
) -> Result<FormTemplate, DbError> {
    let form_type_str = match request.form_type {
        FormType::Admission => "admission",
        FormType::Medical => "medical",
        FormType::Emergency => "emergency",
        FormType::Authorization => "authorization",
        FormType::Handbook => "handbook",
        FormType::Agreement => "agreement",
    };

    // Get next display order
    let max_order = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(MAX(display_order), 0) as "max_order!"
        FROM form_templates
        WHERE school_id = $1 AND is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    let row = sqlx::query!(
        r#"
        INSERT INTO form_templates (
            school_id,
            form_name,
            form_type,
            fillout_form_id,
            fillout_form_url,
            status,
            is_required,
            display_order,
            created_by,
            updated_by
        ) VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, $8, $8)
        RETURNING
            id,
            school_id,
            form_name,
            form_type,
            fillout_form_id,
            fillout_form_url,
            status,
            is_required,
            display_order,
            created_at,
            updated_at
        "#,
        school_id,
        request.form_name,
        form_type_str,
        request.fillout_form_id,
        request.fillout_form_url,
        request.is_required.unwrap_or(false),
        max_order + 1,
        created_by
    )
    .fetch_one(pool)
    .await?;

    Ok(FormTemplate {
        id: row.id,
        school_id: row.school_id,
        form_name: row.form_name,
        form_description: request.form_description,
        form_type: request.form_type,
        fillout_form_id: row.fillout_form_id.unwrap_or_default(),
        fillout_form_url: row.fillout_form_url.unwrap_or_default(),
        status: parse_form_status(&row.status),
        is_required: row.is_required,
        display_order: row.display_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn get_form_submissions_by_enrollment(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    enrollment_id: Option<Uuid>,
    page: i32,
    limit: i32,
) -> Result<Vec<FormSubmission>, DbError> {
    let offset = (page - 1) * limit;

    let rows = if let Some(enrollment_id) = enrollment_id {
        sqlx::query!(
            r#"
            SELECT
                fs.id,
                fs.enrollment_id,
                fs.form_template_id,
                fs.fillout_submission_id,
                fs.form_data,
                fs.submitted_at,
                CASE
                    WHEN e.forms_locked_at IS NOT NULL THEN 'locked'
                    ELSE 'submitted'
                END as status
            FROM form_submissions fs
            JOIN enrollments e ON fs.enrollment_id = e.id
            WHERE fs.school_id = $1
                AND fs.enrollment_id = $2
                AND fs.is_active = true
            ORDER BY fs.submitted_at DESC
            LIMIT $3 OFFSET $4
            "#,
            school_id,
            enrollment_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query!(
            r#"
            SELECT
                fs.id,
                fs.enrollment_id,
                fs.form_template_id,
                fs.fillout_submission_id,
                fs.form_data,
                fs.submitted_at,
                CASE
                    WHEN e.forms_locked_at IS NOT NULL THEN 'locked'
                    ELSE 'submitted'
                END as status
            FROM form_submissions fs
            JOIN enrollments e ON fs.enrollment_id = e.id
            WHERE fs.school_id = $1
                AND fs.is_active = true
            ORDER BY fs.submitted_at DESC
            LIMIT $2 OFFSET $3
            "#,
            school_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?
    };

    let submissions = rows
        .into_iter()
        .map(|row| {
            let form_data = row.form_data
                .and_then(|json| serde_json::from_value(json).ok())
                .unwrap_or_else(|| std::collections::HashMap::new());

            FormSubmission {
                id: row.id,
                enrollment_id: row.enrollment_id,
                form_template_id: row.form_template_id,
                fillout_submission_id: row.fillout_submission_id.unwrap_or_default(),
                form_data,
                status: match row.status.as_deref() {
                    Some("locked") => SubmissionStatus::Locked,
                    Some("revision_needed") => SubmissionStatus::RevisionNeeded,
                    _ => SubmissionStatus::Submitted,
                },
                submitted_at: row.submitted_at.unwrap_or_else(|| chrono::Utc::now()),
                locked_at: None,
            }
        })
        .collect();

    Ok(submissions)
}

pub async fn process_fillout_webhook(
    pool: &Pool<Postgres>,
    request: FilloutWebhookRequest,
) -> Result<(), DbError> {
    // Find the form template by fillout form ID
    let template = sqlx::query!(
        r#"
        SELECT id, school_id
        FROM form_templates
        WHERE fillout_form_id = $1
            AND is_active = true
        "#,
        request.formId
    )
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)?;

    // Extract enrollment_id from form data (this would be passed as a hidden field)
    let enrollment_id = request.data.get("enrollment_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(DbError::InvalidInput("Missing enrollment_id".to_string()))?;

    // Check if submission already exists
    let existing = sqlx::query!(
        r#"
        SELECT id FROM form_submissions
        WHERE fillout_submission_id = $1
        "#,
        request.submissionId
    )
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        // Update existing submission
        sqlx::query!(
            r#"
            UPDATE form_submissions
            SET
                form_data = $2,
                processed_at = NOW(),
                updated_at = NOW()
            WHERE fillout_submission_id = $1
            "#,
            request.submissionId,
            serde_json::to_value(&request.data).unwrap()
        )
        .execute(pool)
        .await?;
    } else {
        // Create new submission
        sqlx::query!(
            r#"
            INSERT INTO form_submissions (
                school_id,
                enrollment_id,
                form_template_id,
                fillout_submission_id,
                form_data,
                submitted_at,
                processed_at
            ) VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            "#,
            template.school_id,
            enrollment_id,
            template.id,
            request.submissionId,
            serde_json::to_value(&request.data).unwrap()
        )
        .execute(pool)
        .await?;
    }

    // Update enrollment progress
    update_enrollment_progress(pool, enrollment_id).await?;

    Ok(())
}

async fn update_enrollment_progress(
    pool: &Pool<Postgres>,
    enrollment_id: Uuid,
) -> Result<(), DbError> {
    // Count total required forms
    let total_forms = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM student_form_assignments
        WHERE enrollment_id = $1
            AND is_active = true
        "#,
        enrollment_id
    )
    .fetch_one(pool)
    .await?;

    // Count submitted forms
    let completed_forms = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM form_submissions
        WHERE enrollment_id = $1
            AND is_active = true
        "#,
        enrollment_id
    )
    .fetch_one(pool)
    .await?;

    let pending_forms = total_forms - completed_forms;
    let completion_percentage = if total_forms > 0 {
        (completed_forms as f64 / total_forms as f64) * 100.0
    } else {
        0.0
    };

    let progress = EnrollmentProgress {
        total_forms: total_forms as i32,
        completed_forms: completed_forms as i32,
        pending_forms: pending_forms as i32,
        completion_percentage,
    };

    // Update enrollment with new progress
    sqlx::query!(
        r#"
        UPDATE enrollments
        SET
            progress = $2,
            status = CASE
                WHEN $3 >= 100.0 THEN 'under_review'
                WHEN $3 > 0 THEN 'in_progress'
                ELSE status
            END,
            updated_at = NOW()
        WHERE id = $1
        "#,
        enrollment_id,
        serde_json::to_value(&progress).unwrap(),
        completion_percentage
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn parse_form_type(form_type: &Option<String>) -> FormType {
    match form_type.as_deref() {
        Some("admission") => FormType::Admission,
        Some("medical") => FormType::Medical,
        Some("emergency") => FormType::Emergency,
        Some("authorization") => FormType::Authorization,
        Some("handbook") => FormType::Handbook,
        Some("agreement") => FormType::Agreement,
        _ => FormType::Admission,
    }
}

fn parse_form_status(status: &str) -> FormStatus {
    match status {
        "draft" => FormStatus::Draft,
        "active" => FormStatus::Active,
        "school_default" => FormStatus::SchoolDefault,
        "archive" => FormStatus::Archive,
        _ => FormStatus::Draft,
    }
}