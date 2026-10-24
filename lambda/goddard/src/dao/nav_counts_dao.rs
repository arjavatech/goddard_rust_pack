use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::nav_counts::{AdminNavCounts, MyNavCounts};

#[derive(Clone)]
pub struct NavCountsDao {
    pool: Pool,
}

impl NavCountsDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Six COUNT(*) subqueries in one round-trip for admin badge counts.
    pub async fn get_admin_counts(&self, school_id: Uuid) -> Result<AdminNavCounts, AppError> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            r#"
            SELECT
              (SELECT COUNT(*) FROM student_form_assignments
               WHERE school_id = $1 AND status = 'incomplete'
                 AND COALESCE(is_active, true) = true)                              AS student_forms_due,
              (SELECT COUNT(*) FROM student_form_assignments
               WHERE school_id = $1 AND status IN ('in_progress', 'manually_uploaded')
                 AND COALESCE(is_active, true) = true)                              AS student_forms_review,
              (SELECT COUNT(*) FROM employee_form_assignments
               WHERE school_id = $1 AND status = 'incomplete'
                 AND COALESCE(is_active, true) = true)                              AS employee_forms_due,
              (SELECT COUNT(*) FROM employee_form_assignments
               WHERE school_id = $1 AND status IN ('in_progress', 'manually_uploaded')
                 AND COALESCE(is_active, true) = true)                              AS employee_forms_review,
              (SELECT COUNT(*) FROM document_request_assignments dra
               JOIN document_requests dr ON dr.id = dra.document_request_id
               WHERE dra.school_id = $1 AND dr.audience = 'student'
                 AND dra.status IN ('pending', 'rejected'))                         AS student_docs_due,
              (SELECT COUNT(*) FROM document_request_assignments dra
               JOIN document_requests dr ON dr.id = dra.document_request_id
               WHERE dra.school_id = $1 AND dr.audience = 'student'
                 AND dra.status = 'submitted')                                      AS student_docs_review,
              (SELECT COUNT(*) FROM document_request_assignments dra
               JOIN document_requests dr ON dr.id = dra.document_request_id
               WHERE dra.school_id = $1 AND dr.audience = 'employee'
                 AND dra.status IN ('pending', 'rejected'))                         AS employee_docs_due,
              (SELECT COUNT(*) FROM document_request_assignments dra
               JOIN document_requests dr ON dr.id = dra.document_request_id
               WHERE dra.school_id = $1 AND dr.audience = 'employee'
                 AND dra.status = 'submitted')                                      AS employee_docs_review
            "#,
            &[&school_id],
        ).await.map_err(|e: tokio_postgres::Error| AppError::Database(e.to_string()))?;

        Ok(AdminNavCounts {
            student_forms_due:    row.get::<_, i64>(0),
            student_forms_review: row.get::<_, i64>(1),
            employee_forms_due:   row.get::<_, i64>(2),
            employee_forms_review: row.get::<_, i64>(3),
            student_docs_due:     row.get::<_, i64>(4),
            student_docs_review:  row.get::<_, i64>(5),
            employee_docs_due:    row.get::<_, i64>(6),
            employee_docs_review: row.get::<_, i64>(7),
        })
    }

    /// Two COUNT(*) subqueries in one round-trip for a parent's badge counts.
    pub async fn get_parent_counts(&self, user_id: Uuid, school_id: Uuid) -> Result<MyNavCounts, AppError> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            r#"
            SELECT
              (SELECT COUNT(*) FROM student_form_assignments sfa
               JOIN children c ON c.id = sfa.child_id
               WHERE (c.parent_id = $1 OR c.secondary_parent_id = $1)
                 AND sfa.school_id = $2 AND sfa.status = 'incomplete'
                 AND COALESCE(sfa.is_active, true) = true)   AS forms_pending,
              (SELECT COUNT(*) FROM document_request_assignments dra
               JOIN children c ON c.id = dra.child_id
               WHERE (c.parent_id = $1 OR c.secondary_parent_id = $1)
                 AND dra.school_id = $2 AND dra.status = 'pending') AS docs_pending
            "#,
            &[&user_id, &school_id],
        ).await.map_err(|e: tokio_postgres::Error| AppError::Database(e.to_string()))?;

        Ok(MyNavCounts {
            forms_pending: row.get::<_, i64>(0),
            docs_pending:  row.get::<_, i64>(1),
        })
    }

    /// Two COUNT(*) subqueries in one round-trip for an employee's badge counts.
    pub async fn get_employee_counts(&self, user_id: Uuid, school_id: Uuid) -> Result<MyNavCounts, AppError> {
        let client = self.pool.get().await
            .map_err(|e: deadpool_postgres::PoolError| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            r#"
            SELECT
              (SELECT COUNT(*) FROM employee_form_assignments
               WHERE user_id = $1 AND school_id = $2 AND status = 'incomplete'
                 AND COALESCE(is_active, true) = true)                AS forms_pending,
              (SELECT COUNT(*) FROM document_request_assignments dra
               JOIN employees e ON e.id = dra.employee_id
               WHERE e.user_id = $1 AND dra.school_id = $2
                 AND dra.status = 'pending')                          AS docs_pending
            "#,
            &[&user_id, &school_id],
        ).await.map_err(|e: tokio_postgres::Error| AppError::Database(e.to_string()))?;

        Ok(MyNavCounts {
            forms_pending: row.get::<_, i64>(0),
            docs_pending:  row.get::<_, i64>(1),
        })
    }
}
