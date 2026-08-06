use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::models::employee::{EmployeeFormAssignment, EmployeeFormAssignmentWithTemplate};
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

    pub async fn get_assignments_by_employee(&self, employee_id: Uuid) -> Result<Vec<EmployeeFormAssignmentWithTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT a.id, a.school_id, a.employee_id, a.user_id, a.employee_form_template_id,
                    t.form_name, t.fillout_form_id, t.due_date,
                    a.assignment_source, a.status, a.is_required, a.assigned_by, a.assigned_at,
                    a.approved_by, a.approved_on, a.notes, a.recent_edit_link, a.recent_pdf_link,
                    u.first_name as employee_first_name, u.last_name as employee_last_name
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
                    u.first_name as employee_first_name, u.last_name as employee_last_name
             FROM employee_form_assignments a
             JOIN employee_form_templates t ON a.employee_form_template_id = t.id
             JOIN users u ON a.user_id = u.id
             WHERE a.school_id = $1 AND (a.is_active = true OR a.is_active IS NULL)
             ORDER BY a.assigned_at DESC",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch assignments by school: {}", e)))?;

        Ok(rows.iter().map(Self::row_to_assignment_with_template).collect())
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
}
