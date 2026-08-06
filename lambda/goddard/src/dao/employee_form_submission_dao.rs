use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::models::employee::EmployeeFormSubmission;
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct EmployeeFormSubmissionDao {
    pool: Pool,
}

impl EmployeeFormSubmissionDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_submission(row: &tokio_postgres::Row) -> EmployeeFormSubmission {
        EmployeeFormSubmission {
            id: row.get("id"),
            school_id: row.get("school_id"),
            employee_id: row.get("employee_id"),
            employee_form_assignment_id: row.get("employee_form_assignment_id"),
            employee_form_template_id: row.get("employee_form_template_id"),
            fillout_submission_id: row.get("fillout_submission_id"),
            form_data: row.get("form_data"),
            metadata: row.get("metadata"),
            status: row.get("status"),
            revision_number: row.get("revision_number"),
            edit_link: row.get("edit_link"),
            pdf_link: row.get("pdf_link"),
            submitted_at: row.get("submitted_at"),
            created_at: row.get("created_at"),
        }
    }

    pub async fn upsert_submission(
        &self,
        school_id: Uuid,
        employee_id: Uuid,
        assignment_id: Uuid,
        template_id: Uuid,
        fillout_submission_id: &str,
        form_data: Option<&serde_json::Value>,
        metadata: Option<&serde_json::Value>,
        edit_link: Option<&str>,
        pdf_link: Option<&str>,
    ) -> Result<EmployeeFormSubmission, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "INSERT INTO employee_form_submissions
             (id, school_id, employee_id, employee_form_assignment_id, employee_form_template_id,
              fillout_submission_id, form_data, metadata, status, edit_link, pdf_link,
              submitted_at, is_active, created_at, updated_at)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, 'completed', $8, $9, NOW(), true, NOW(), NOW())
             ON CONFLICT (fillout_submission_id) DO UPDATE SET
               form_data = EXCLUDED.form_data, metadata = EXCLUDED.metadata,
               edit_link = COALESCE(EXCLUDED.edit_link, employee_form_submissions.edit_link),
               pdf_link = COALESCE(EXCLUDED.pdf_link, employee_form_submissions.pdf_link),
               revision_number = employee_form_submissions.revision_number + 1,
               updated_at = NOW()
             RETURNING id, school_id, employee_id, employee_form_assignment_id, employee_form_template_id,
                       fillout_submission_id, form_data, metadata, status, revision_number,
                       edit_link, pdf_link, submitted_at, created_at",
            &[&school_id, &employee_id, &assignment_id, &template_id,
              &fillout_submission_id, &form_data, &metadata, &edit_link, &pdf_link],
        ).await.map_err(|e| AppError::Database(format!("Failed to upsert employee form submission: {}", e)))?;

        Ok(Self::row_to_submission(&row))
    }

    pub async fn get_submission_by_assignment(&self, assignment_id: Uuid) -> Result<Option<EmployeeFormSubmission>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT id, school_id, employee_id, employee_form_assignment_id, employee_form_template_id,
                    fillout_submission_id, form_data, metadata, status, revision_number,
                    edit_link, pdf_link, submitted_at, created_at
             FROM employee_form_submissions WHERE employee_form_assignment_id = $1
             ORDER BY revision_number DESC LIMIT 1",
            &[&assignment_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch submission: {}", e)))?;

        Ok(row.map(|r| Self::row_to_submission(&r)))
    }

    pub async fn get_submissions_by_employee(&self, employee_id: Uuid) -> Result<Vec<EmployeeFormSubmission>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT id, school_id, employee_id, employee_form_assignment_id, employee_form_template_id,
                    fillout_submission_id, form_data, metadata, status, revision_number,
                    edit_link, pdf_link, submitted_at, created_at
             FROM employee_form_submissions WHERE employee_id = $1
             ORDER BY created_at DESC",
            &[&employee_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch submissions by employee: {}", e)))?;

        Ok(rows.iter().map(Self::row_to_submission).collect())
    }
}
