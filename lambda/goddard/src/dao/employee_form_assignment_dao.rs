use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::models::employee::{EmployeeFormAssignment, EmployeeFormAssignmentWithTemplate};
use crate::models::form_review_queue::EmployeeFormReviewQueueItem;
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct EmployeeFormAssignmentDao {
    pool: Pool,
}

impl EmployeeFormAssignmentDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_assignment(row: &tokio_postgres::Row) -> EmployeeFormAssignment {
        EmployeeFormAssignment {
            id: row.get("id"),
            school_id: row.get("school_id"),
            employee_id: row.get("employee_id"),
            user_id: row.get("user_id"),
            employee_form_template_id: row.get("employee_form_template_id"),
            assignment_source: row.get("assignment_source"),
            status: row.get("status"),
            is_required: row.get("is_required"),
            assigned_by: row.get("assigned_by"),
            assigned_at: row.get("assigned_at"),
            approved_by: row.get("approved_by"),
            approved_on: row.get("approved_on"),
            notes: row.get("notes"),
            recent_edit_link: row.get("recent_edit_link"),
            recent_pdf_link: row.get("recent_pdf_link"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            submission_source: row.try_get("submission_source").ok(),
            manual_pdf_storage_key: row.try_get("manual_pdf_storage_key").ok().flatten(),
            manual_pdf_file_name: row.try_get("manual_pdf_file_name").ok().flatten(),
            manual_pdf_content_type: row.try_get("manual_pdf_content_type").ok().flatten(),
            manual_pdf_file_size_bytes: row.try_get("manual_pdf_file_size_bytes").ok().flatten(),
            manual_pdf_uploaded_at: row.try_get("manual_pdf_uploaded_at").ok().flatten(),
            manual_pdf_uploaded_by: row.try_get("manual_pdf_uploaded_by").ok().flatten(),
        }
    }

    fn row_to_assignment_with_template(row: &tokio_postgres::Row) -> EmployeeFormAssignmentWithTemplate {
        EmployeeFormAssignmentWithTemplate {
            id: row.get("id"),
            school_id: row.get("school_id"),
            employee_id: row.get("employee_id"),
            user_id: row.get("user_id"),
            employee_form_template_id: row.get("employee_form_template_id"),
            form_name: row.get("form_name"),
            fillout_form_id: row.get("fillout_form_id"),
            due_date: row.get("due_date"),
            assignment_source: row.get("assignment_source"),
            status: row.get("status"),
            is_required: row.get("is_required"),
            assigned_by: row.get("assigned_by"),
            assigned_at: row.get("assigned_at"),
            approved_by: row.get("approved_by"),
            approved_on: row.get("approved_on"),
            notes: row.get("notes"),
            recent_edit_link: row.get("recent_edit_link"),
            recent_pdf_link: row.get("recent_pdf_link"),
            employee_first_name: row.get("employee_first_name"),
            employee_last_name: row.get("employee_last_name"),
            submission_source: row.try_get("submission_source").ok(),
            manual_pdf_storage_key: row.try_get("manual_pdf_storage_key").ok().flatten(),
            manual_pdf_file_name: row.try_get("manual_pdf_file_name").ok().flatten(),
            manual_pdf_content_type: row.try_get("manual_pdf_content_type").ok().flatten(),
            manual_pdf_file_size_bytes: row.try_get("manual_pdf_file_size_bytes").ok().flatten(),
            manual_pdf_uploaded_at: row.try_get("manual_pdf_uploaded_at").ok().flatten(),
            manual_pdf_uploaded_by: row.try_get("manual_pdf_uploaded_by").ok().flatten(),
        }
    }

    pub async fn create_assignment(
        &self,
        employee_id: Uuid,
        user_id: Uuid,
        school_id: Uuid,
        template_id: Uuid,
        assigned_by: Uuid,
        is_required: bool,
    ) -> Result<EmployeeFormAssignment, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "INSERT INTO employee_form_assignments
             (id, school_id, employee_id, user_id, employee_form_template_id,
              assignment_source, status, is_required, assigned_by, assigned_at, is_active, created_at, updated_at)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, 'manual', 'incomplete', $5, $6, NOW(), true, NOW(), NOW())
             RETURNING id, school_id, employee_id, user_id, employee_form_template_id,
                       assignment_source, status, is_required, assigned_by, assigned_at,
                       approved_by, approved_on, notes, recent_edit_link, recent_pdf_link,
                       is_active, created_at, updated_at",
            &[&school_id, &employee_id, &user_id, &template_id, &is_required, &assigned_by],
        ).await.map_err(|e| AppError::Database(format!("Failed to create employee form assignment: {}", e)))?;

        Ok(Self::row_to_assignment(&row))
    }

    pub async fn assign_template_to_school_employees(
        &self,
        school_id: Uuid,
        template_id: Uuid,
        assigned_by: Uuid,
        is_required: bool,
    ) -> Result<(i64, i64, i64), AppError> {
        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        let total_row = transaction.query_one(
            "SELECT COUNT(*) FROM employees WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to count active employees: {}", e)))?;
        let total_active_employees: i64 = total_row.get(0);

        let assigned_row = transaction.query_one(
            "SELECT COUNT(DISTINCT e.id)
             FROM employees e
             JOIN employee_form_assignments a ON a.employee_id = e.id
             WHERE e.school_id = $1
               AND a.employee_form_template_id = $2
               AND (e.is_active = true OR e.is_active IS NULL)
               AND (a.is_active = true OR a.is_active IS NULL)",
            &[&school_id, &template_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to count existing employee assignments: {}", e)))?;
        let employees_already_assigned: i64 = assigned_row.get(0);

        let rows = transaction.query(
            "INSERT INTO employee_form_assignments
             (id, school_id, employee_id, user_id, employee_form_template_id,
              assignment_source, status, is_required, assigned_by, assigned_at, is_active, created_at, updated_at)
             SELECT gen_random_uuid(), e.school_id, e.id, e.user_id, $2,
                    'school_default', 'incomplete', $3, $4, NOW(), true, NOW(), NOW()
             FROM employees e
             WHERE e.school_id = $1
               AND (e.is_active = true OR e.is_active IS NULL)
               AND NOT EXISTS (
                   SELECT 1 FROM employee_form_assignments a
                   WHERE a.employee_id = e.id
                     AND a.employee_form_template_id = $2
                     AND (a.is_active = true OR a.is_active IS NULL)
               )
             ON CONFLICT (employee_id, employee_form_template_id) DO NOTHING
             RETURNING id",
            &[&school_id, &template_id, &is_required, &assigned_by],
        ).await.map_err(|e| AppError::Database(format!("Failed to assign form to employees: {}", e)))?;

        let newly_assigned = rows.len() as i64;
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit employee form assignments: {}", e)))?;

        Ok((total_active_employees, employees_already_assigned, newly_assigned))
    }

    pub async fn get_assignments_by_employee(&self, employee_id: Uuid) -> Result<Vec<EmployeeFormAssignmentWithTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT a.id, a.school_id, a.employee_id, a.user_id, a.employee_form_template_id,
                    t.form_name, t.fillout_form_id, t.due_date,
                    a.assignment_source, a.status, a.is_required, a.assigned_by, a.assigned_at,
                    a.approved_by, a.approved_on, a.notes, a.recent_edit_link, a.recent_pdf_link,
                    u.first_name as employee_first_name, u.last_name as employee_last_name,
                    a.submission_source, a.manual_pdf_storage_key, a.manual_pdf_file_name,
                    a.manual_pdf_content_type, a.manual_pdf_file_size_bytes, a.manual_pdf_uploaded_at, a.manual_pdf_uploaded_by
             FROM employee_form_assignments a
             JOIN employee_form_templates t ON a.employee_form_template_id = t.id
             JOIN users u ON a.user_id = u.id
             WHERE a.employee_id = $1 AND (a.is_active = true OR a.is_active IS NULL)
             ORDER BY a.assigned_at DESC",
            &[&employee_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch assignments by employee: {}", e)))?;

        Ok(rows.iter().map(Self::row_to_assignment_with_template).collect())
    }

    pub async fn get_assignments_by_school(&self, school_id: Uuid) -> Result<Vec<EmployeeFormAssignmentWithTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT a.id, a.school_id, a.employee_id, a.user_id, a.employee_form_template_id,
                    t.form_name, t.fillout_form_id, t.due_date,
                    a.assignment_source, a.status, a.is_required, a.assigned_by, a.assigned_at,
                    a.approved_by, a.approved_on, a.notes, a.recent_edit_link, a.recent_pdf_link,
                    u.first_name as employee_first_name, u.last_name as employee_last_name,
                    a.submission_source, a.manual_pdf_storage_key, a.manual_pdf_file_name,
                    a.manual_pdf_content_type, a.manual_pdf_file_size_bytes, a.manual_pdf_uploaded_at, a.manual_pdf_uploaded_by
             FROM employee_form_assignments a
             JOIN employee_form_templates t ON a.employee_form_template_id = t.id
             JOIN users u ON a.user_id = u.id
             WHERE a.school_id = $1 AND (a.is_active = true OR a.is_active IS NULL)
             ORDER BY a.assigned_at DESC",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch assignments by school: {}", e)))?;

        Ok(rows.iter().map(Self::row_to_assignment_with_template).collect())
    }

    pub async fn get_review_queue(&self, school_id: Uuid) -> Result<Vec<EmployeeFormReviewQueueItem>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let rows = client.query(
            r#"
            SELECT a.id AS assignment_id, a.school_id, a.employee_id, a.employee_form_template_id AS form_template_id,
                   t.form_name, t.fillout_form_id, a.status,
                   COALESCE(s.submitted_at, a.updated_at, a.assigned_at) AS submitted_at,
                   COALESCE(s.edit_link, a.recent_edit_link) AS recent_edit_link,
                   COALESCE(s.pdf_link, a.recent_pdf_link) AS recent_pdf_link,
                   u.first_name AS employee_first_name, u.last_name AS employee_last_name, u.email AS employee_email,
                   a.submission_source, a.manual_pdf_uploaded_at
            FROM employee_form_assignments a
            JOIN employee_form_templates t ON t.id = a.employee_form_template_id
            JOIN users u ON u.id = a.user_id
            LEFT JOIN LATERAL (
                SELECT submitted_at, edit_link, pdf_link
                FROM employee_form_submissions
                WHERE employee_form_assignment_id = a.id
                  AND (is_active = true OR is_active IS NULL)
                ORDER BY submitted_at DESC
                LIMIT 1
            ) s ON true
            WHERE a.school_id = $1
              AND a.status IN ('in_progress', 'manually_uploaded')
              AND (a.is_active = true OR a.is_active IS NULL)
            ORDER BY COALESCE(s.submitted_at, a.updated_at, a.assigned_at) DESC
            "#,
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee review queue: {}", e)))?;
        Ok(rows.into_iter().map(|row| EmployeeFormReviewQueueItem {
            assignment_id: row.get("assignment_id"), school_id: row.get("school_id"), employee_id: row.get("employee_id"),
            form_template_id: row.get("form_template_id"), form_name: row.get("form_name"), fillout_form_id: row.get("fillout_form_id"),
            status: row.get("status"), submitted_at: row.get("submitted_at"), recent_edit_link: row.get("recent_edit_link"),
            recent_pdf_link: row.get("recent_pdf_link"), employee_first_name: row.get("employee_first_name"),
            employee_last_name: row.get("employee_last_name"), employee_email: row.get("employee_email"),
            submission_source: row.try_get("submission_source").ok(),
            manual_pdf_uploaded_at: row.try_get("manual_pdf_uploaded_at").ok(),
        }).collect())
    }

    pub async fn update_assignment_status(
        &self,
        assignment_id: Uuid,
        status: &str,
        edit_link: Option<&str>,
        pdf_link: Option<&str>,
    ) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        client.execute(
            "UPDATE employee_form_assignments
             SET status = $2, recent_edit_link = COALESCE($3, recent_edit_link),
                 recent_pdf_link = COALESCE($4, recent_pdf_link), updated_at = NOW()
             WHERE id = $1",
            &[&assignment_id, &status, &edit_link, &pdf_link],
        ).await.map_err(|e| AppError::Database(format!("Failed to update assignment status: {}", e)))?;

        Ok(())
    }

    pub async fn review_assignment(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
        status: &str,
        approved_by: Uuid,
        notes: Option<&str>,
    ) -> Result<EmployeeFormAssignment, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "UPDATE employee_form_assignments
             SET status = $3, approved_by = $4, approved_on = NOW(), notes = $5, updated_at = NOW()
             WHERE id = $1 AND school_id = $2
             RETURNING id, school_id, employee_id, user_id, employee_form_template_id,
                       assignment_source, status, is_required, assigned_by, assigned_at,
                       approved_by, approved_on, notes, recent_edit_link, recent_pdf_link,
                       is_active, created_at, updated_at",
            &[&assignment_id, &school_id, &status, &approved_by, &notes],
        ).await.map_err(|e| AppError::Database(format!("Failed to review assignment: {}", e)))?;

        if rows.is_empty() {
            return Err(AppError::NotFound("Employee form assignment not found".to_string()));
        }
        Ok(Self::row_to_assignment(&rows[0]))
    }

    pub async fn get_assignment_details(&self, assignment_id: Uuid) -> Result<Option<(Uuid, Uuid, Uuid)>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT school_id, employee_id, employee_form_template_id
             FROM employee_form_assignments WHERE id = $1 LIMIT 1",
            &[&assignment_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get assignment details: {}", e)))?;

        Ok(row.map(|r| (r.get("school_id"), r.get("employee_id"), r.get("employee_form_template_id"))))
    }

    pub async fn delete_assignment(&self, assignment_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let n = client.execute(
            "UPDATE employee_form_assignments SET is_active = false, updated_at = NOW() WHERE id = $1 AND school_id = $2",
            &[&assignment_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to delete assignment: {}", e)))?;

        if n == 0 { return Err(AppError::NotFound("Employee form assignment not found".to_string())); }
        Ok(())
    }

    pub async fn complete_manual_pdf_upload(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
        storage_key: &str,
        file_name: &str,
        content_type: &str,
        file_size_bytes: i64,
        uploaded_by: &str,
    ) -> Result<EmployeeFormAssignment, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            r#"
            UPDATE employee_form_assignments
            SET status = 'manually_uploaded',
                manual_pdf_storage_key = $3,
                manual_pdf_file_name = $4,
                manual_pdf_content_type = $5,
                manual_pdf_file_size_bytes = $6,
                manual_pdf_uploaded_at = NOW(),
                manual_pdf_uploaded_by = $7,
                submission_source = 'manual_upload',
                updated_at = NOW()
            WHERE id = $1 AND school_id = $2
            RETURNING id, school_id, employee_id, user_id, employee_form_template_id,
                      assignment_source, status, is_required, assigned_by, assigned_at,
                      approved_by, approved_on, notes, recent_edit_link, recent_pdf_link,
                      is_active, created_at, updated_at,
                      submission_source, manual_pdf_storage_key, manual_pdf_file_name,
                      manual_pdf_content_type, manual_pdf_file_size_bytes, manual_pdf_uploaded_at,
                      manual_pdf_uploaded_by
            "#,
            &[
                &assignment_id,
                &school_id,
                &storage_key,
                &file_name,
                &content_type,
                &file_size_bytes,
                &uploaded_by,
            ],
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to complete manual PDF upload: {}", e)))?;

        Ok(Self::row_to_assignment(&row))
    }

    pub async fn get_manual_pdf_storage_key(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT manual_pdf_storage_key FROM employee_form_assignments WHERE id = $1 AND school_id = $2",
            &[&assignment_id, &school_id],
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to get manual PDF storage key: {}", e)))?;

        Ok(row.and_then(|r| r.try_get("manual_pdf_storage_key").ok()))
    }

    pub async fn remove_manual_pdf(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
    ) -> Result<EmployeeFormAssignment, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            r#"
            UPDATE employee_form_assignments
            SET status = 'incomplete',
                manual_pdf_storage_key = NULL,
                manual_pdf_file_name = NULL,
                manual_pdf_content_type = NULL,
                manual_pdf_file_size_bytes = NULL,
                manual_pdf_uploaded_at = NULL,
                manual_pdf_uploaded_by = NULL,
                submission_source = 'digital',
                updated_at = NOW()
            WHERE id = $1 AND school_id = $2
            RETURNING id, school_id, employee_id, user_id, employee_form_template_id,
                      assignment_source, status, is_required, assigned_by, assigned_at,
                      approved_by, approved_on, notes, recent_edit_link, recent_pdf_link,
                      is_active, created_at, updated_at,
                      submission_source, manual_pdf_storage_key, manual_pdf_file_name,
                      manual_pdf_content_type, manual_pdf_file_size_bytes, manual_pdf_uploaded_at,
                      manual_pdf_uploaded_by
            "#,
            &[&assignment_id, &school_id],
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to remove manual PDF upload: {}", e)))?;

        Ok(Self::row_to_assignment(&row))
    }
}
