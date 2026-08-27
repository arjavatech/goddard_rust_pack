use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use crate::models::form_template::{FormTemplate, CreateFormTemplateRequest, UpdateFormTemplateRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct FormTemplateDao {
    pool: Pool,
}

impl FormTemplateDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_form_template(row: &Row) -> FormTemplate {
        FormTemplate {
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

    pub async fn create_form_template(&self, request: &CreateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Extract status or default to 'school_default' for backward compatibility
        let status = request.status.as_deref().unwrap_or("school_default");

        let query = r#"
            INSERT INTO form_templates (id, school_id, form_name, fillout_form_id, due_date, status, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, true, NOW(), NOW())
            RETURNING id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at
        "#;

        let stmt = client.prepare(query).await
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let row = client.query_one(&stmt, &[&request.school_id, &request.form_name, &request.fillout_form_id, &request.due_date, &status])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create form template: {}", e)))?;

        Ok(Self::row_to_form_template(&row))
    }

    pub async fn get_form_templates_by_school(&self, school_id: &Uuid) -> Result<Vec<FormTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at
            FROM form_templates
            WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
            ORDER BY form_name ASC
        "#;

        let stmt = client.prepare(query).await
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = client.query(&stmt, &[school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch form templates: {}", e)))?;

        let form_templates = rows.iter().map(Self::row_to_form_template).collect();
        Ok(form_templates)
    }

    pub async fn get_form_template_by_id(&self, id: Uuid, school_id: Uuid) -> Result<Option<FormTemplate>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client.query_opt(
            "SELECT id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at FROM form_templates WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch form template: {}", e)))?;
        Ok(row.map(|r| Self::row_to_form_template(&r)))
    }

    pub async fn update_form_template(&self, request: &UpdateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE form_templates
            SET form_name = $3, form_type = $4, fillout_form_id = $5, due_date = $6, status = $7, is_required = $8, display_order = $9, updated_at = NOW()
            WHERE id = $2 AND school_id = $1 AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at
        "#;

        let stmt = client.prepare(query).await
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = client.query(&stmt, &[&request.school_id, &request.id, &request.form_name, &request.form_type, &request.fillout_form_id, &request.due_date, &request.status, &request.is_required, &request.display_order])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update form template: {}", e)))?;

        if rows.is_empty() {
            return Err(AppError::NotFound("Form template not found".to_string()));
        }

        Ok(Self::row_to_form_template(&rows[0]))
    }

    pub async fn set_pdf(&self, id: Uuid, school_id: Uuid, key: &str, file_name: &str, content_type: &str, size: i64) -> Result<FormTemplate, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client.query_opt(
            "UPDATE form_templates SET pdf_storage_key = $3, pdf_file_name = $4, pdf_content_type = $5, pdf_file_size_bytes = $6, pdf_uploaded_at = NOW(), updated_at = NOW() WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL) RETURNING id, school_id, form_name, form_type, fillout_form_id, pdf_storage_key, pdf_file_name, pdf_content_type, pdf_file_size_bytes, pdf_uploaded_at, due_date, status, is_required, display_order, is_active, created_at, updated_at",
            &[&id, &school_id, &key, &file_name, &content_type, &size],
        ).await.map_err(|e| AppError::Database(format!("Failed to attach form template PDF: {}", e)))?;
        row.map(|r| Self::row_to_form_template(&r)).ok_or_else(|| AppError::NotFound("Form template not found".into()))
    }

    pub async fn clear_pdf(&self, id: Uuid, school_id: Uuid) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client.query_opt(
            "SELECT pdf_storage_key FROM form_templates WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to remove form template PDF: {}", e)))?;
        let key = row.map(|r| r.get::<_, Option<String>>("pdf_storage_key")).ok_or_else(|| AppError::NotFound("Form template not found".into()))?;
        client.execute("UPDATE form_templates SET pdf_storage_key = NULL, pdf_file_name = NULL, pdf_content_type = NULL, pdf_file_size_bytes = NULL, pdf_uploaded_at = NULL, updated_at = NOW() WHERE id = $1 AND school_id = $2", &[&id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to remove form template PDF: {}", e)))?;
        Ok(key)
    }

    /// Look up just the template name by id (used by notifications to render
    /// "Form 'X' deleted" before the delete UPDATE runs).
    pub async fn get_form_template_name(&self, form_template_id: &Uuid) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client
            .query_opt("SELECT form_name FROM form_templates WHERE id = $1", &[form_template_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch form template name: {}", e)))?;
        Ok(row.map(|r| r.get::<_, String>("form_name")))
    }

    pub async fn delete_form_template(&self, form_template_id: &Uuid, school_id: &Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE form_templates
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND school_id = $2
        "#;

        let stmt = client.prepare(query).await
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows_affected = client.execute(&stmt, &[form_template_id, school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete form template: {}", e)))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Form template not found".to_string()));
        }

        Ok(())
    }
}
