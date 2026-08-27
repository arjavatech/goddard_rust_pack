use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::models::employee::{EmployeeFormTemplate, CreateEmployeeFormTemplateRequest, UpdateEmployeeFormTemplateRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct EmployeeFormTemplateDao {
    pool: Pool,
}

impl EmployeeFormTemplateDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_template(row: &tokio_postgres::Row) -> EmployeeFormTemplate {
        EmployeeFormTemplate {
            id: row.get("id"),
            school_id: row.get("school_id"),
            form_name: row.get("form_name"),
            form_type: row.get("form_type"),
            fillout_form_id: row.get("fillout_form_id"),
            pdf_storage_key: row.get("pdf_storage_key"),
            pdf_file_name: row.get("pdf_file_name"),
            pdf_content_type: row.get("pdf_content_type"),
            pdf_file_size_bytes: row.get("pdf_file_size_bytes"),
            pdf_uploaded_at: row.get("pdf_uploaded_at"),
            due_date: row.get("due_date"),
            status: row.get("status"),
            is_required: row.get("is_required"),
            display_order: row.get("display_order"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn create_template(&self, req: &CreateEmployeeFormTemplateRequest) -> Result<EmployeeFormTemplate, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let status = req.status.as_deref().unwrap_or("active");

        let row = client.query_one(
            "INSERT INTO employee_form_templates (id, school_id, form_name, form_type, fillout_form_id, due_date, status, is_required, display_order, is_active, created_at, updated_at)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, true, NOW(), NOW())
             RETURNING id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at",
            &[&req.school_id, &req.form_name, &req.form_type, &req.fillout_form_id, &req.due_date, &status, &req.is_required, &req.display_order],
        ).await.map_err(|e| AppError::Database(format!("Failed to create employee form template: {}", e)))?;

        Ok(Self::row_to_template(&row))
    }

    pub async fn get_templates_by_school(&self, school_id: Uuid) -> Result<Vec<EmployeeFormTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at
             FROM employee_form_templates
             WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
             ORDER BY form_name ASC",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee form templates: {}", e)))?;

        Ok(rows.iter().map(Self::row_to_template).collect())
    }

    pub async fn get_template_by_id(&self, id: Uuid, school_id: Uuid) -> Result<Option<EmployeeFormTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at
             FROM employee_form_templates WHERE id = $1 AND school_id = $2",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee form template: {}", e)))?;

        Ok(row.map(|r| Self::row_to_template(&r)))
    }

    pub async fn update_template(&self, req: &UpdateEmployeeFormTemplateRequest) -> Result<EmployeeFormTemplate, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "UPDATE employee_form_templates
             SET form_name = $3, form_type = $4, fillout_form_id = $5, due_date = $6,
                 status = $7, is_required = $8, display_order = $9, updated_at = NOW()
             WHERE id = $2 AND school_id = $1 AND (is_active = true OR is_active IS NULL)
             RETURNING id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at",
            &[&req.school_id, &req.id, &req.form_name, &req.form_type, &req.fillout_form_id,
              &req.due_date, &req.status, &req.is_required, &req.display_order],
        ).await.map_err(|e| AppError::Database(format!("Failed to update employee form template: {}", e)))?;

        if rows.is_empty() {
            return Err(AppError::NotFound("Employee form template not found".to_string()));
        }
        Ok(Self::row_to_template(&rows[0]))
    }

    pub async fn set_pdf(&self, id: Uuid, school_id: Uuid, key: &str, file_name: &str, content_type: &str, size: i64) -> Result<EmployeeFormTemplate, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client.query_opt(
            "UPDATE employee_form_templates SET pdf_storage_key = $3, pdf_file_name = $4, pdf_content_type = $5, pdf_file_size_bytes = $6, pdf_uploaded_at = NOW(), updated_at = NOW() WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL) RETURNING id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at",
            &[&id, &school_id, &key, &file_name, &content_type, &size],
        ).await.map_err(|e| AppError::Database(format!("Failed to attach employee form template PDF: {}", e)))?;
        row.map(|r| Self::row_to_template(&r)).ok_or_else(|| AppError::NotFound("Employee form template not found".into()))
    }

    pub async fn clear_pdf(&self, id: Uuid, school_id: Uuid) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client.query_opt(
            "SELECT pdf_storage_key FROM employee_form_templates WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to remove employee form template PDF: {}", e)))?;
        let key = row.map(|r| r.get::<_, Option<String>>("pdf_storage_key")).ok_or_else(|| AppError::NotFound("Employee form template not found".into()))?;
        client.execute("UPDATE employee_form_templates SET pdf_storage_key = NULL, pdf_file_name = NULL, pdf_content_type = NULL, pdf_file_size_bytes = NULL, pdf_uploaded_at = NULL, updated_at = NOW() WHERE id = $1 AND school_id = $2", &[&id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to remove employee form template PDF: {}", e)))?;
        Ok(key)
    }

    pub async fn delete_template(&self, form_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let n = client.execute(
            "UPDATE employee_form_templates SET is_active = false, updated_at = NOW() WHERE id = $1 AND school_id = $2",
            &[&form_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to delete employee form template: {}", e)))?;

        if n == 0 { return Err(AppError::NotFound("Employee form template not found".to_string())); }
        Ok(())
    }
}
