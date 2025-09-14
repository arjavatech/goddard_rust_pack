use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::schema::{DashboardOverview, EnrollmentStats, FormStats, DocumentStats};
use super::DbError;

pub async fn get_dashboard_overview(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<DashboardOverview, DbError> {
    // Get enrollment statistics
    let enrollment_stats = get_enrollment_statistics(pool, school_id).await?;

    // Get form statistics
    let form_stats = get_form_statistics(pool, school_id).await?;

    // Get document statistics
    let document_stats = get_document_statistics(pool, school_id).await?;

    // Get total children count
    let total_children = sqlx::query_scalar!(
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

    // Get total classrooms count
    let total_classrooms = sqlx::query_scalar!(
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

    Ok(DashboardOverview {
        total_enrollments: enrollment_stats.total,
        pending_enrollments: enrollment_stats.pending,
        approved_enrollments: enrollment_stats.approved,
        rejected_enrollments: enrollment_stats.rejected,
        total_children: total_children as i32,
        total_classrooms: total_classrooms as i32,
        enrollment_stats,
        form_stats,
        document_stats,
    })
}

async fn get_enrollment_statistics(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<EnrollmentStats, DbError> {
    let stats = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as "total!",
            COUNT(CASE WHEN status = 'pending' THEN 1 END) as "pending!",
            COUNT(CASE WHEN status = 'in_progress' THEN 1 END) as "in_progress!",
            COUNT(CASE WHEN status = 'under_review' THEN 1 END) as "under_review!",
            COUNT(CASE WHEN status = 'approved' THEN 1 END) as "approved!",
            COUNT(CASE WHEN status = 'rejected' THEN 1 END) as "rejected!",
            COUNT(CASE WHEN status = 'needs_revision' THEN 1 END) as "needs_revision!"
        FROM enrollments
        WHERE school_id = $1
            AND is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    Ok(EnrollmentStats {
        total: stats.total as i32,
        pending: stats.pending as i32,
        in_progress: stats.in_progress as i32,
        under_review: stats.under_review as i32,
        approved: stats.approved as i32,
        rejected: stats.rejected as i32,
        needs_revision: stats.needs_revision as i32,
    })
}

async fn get_form_statistics(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<FormStats, DbError> {
    let template_stats = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as "total_templates!",
            COUNT(CASE WHEN status = 'active' THEN 1 END) as "active_templates!"
        FROM form_templates
        WHERE school_id = $1
            AND is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    let submission_stats = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as "total_submissions!"
        FROM form_submissions fs
        WHERE fs.school_id = $1
            AND fs.is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    Ok(FormStats {
        total_templates: template_stats.total_templates as i32,
        active_templates: template_stats.active_templates as i32,
        total_submissions: submission_stats.total_submissions as i32,
        completed_submissions: submission_stats.total_submissions as i32, // All submissions are completed
    })
}

async fn get_document_statistics(
    pool: &Pool<Postgres>,
    school_id: Uuid,
) -> Result<DocumentStats, DbError> {
    let stats = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as "total!",
            COUNT(CASE WHEN status = 'pending' THEN 1 END) as "pending!",
            COUNT(CASE WHEN status = 'approved' THEN 1 END) as "approved!",
            COUNT(CASE WHEN status = 'rejected' THEN 1 END) as "rejected!"
        FROM documents
        WHERE school_id = $1
            AND is_active = true
        "#,
        school_id
    )
    .fetch_one(pool)
    .await?;

    Ok(DocumentStats {
        total: stats.total as i32,
        pending: stats.pending as i32,
        approved: stats.approved as i32,
        rejected: stats.rejected as i32,
    })
}