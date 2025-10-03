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

        let query = r#"
            INSERT INTO form_templates (id, school_id, form_name, fillout_form_id, status, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, 'school_default', true, NOW(), NOW())
            RETURNING id, school_id, form_name, form_type, fillout_form_id, status, is_required, display_order, is_active, created_at, updated_at
        "#;

        let stmt = client.prepare(query).await
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let row = client.query_one(&stmt, &[&request.school_id, &request.form_name, &request.fillout_form_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create form template: {}", e)))?;

        Ok(Self::row_to_form_template(&row))
    }

    pub async fn get_form_templates_by_school(&self, school_id: &Uuid) -> Result<Vec<FormTemplate>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT id, school_id, form_name, form_type, fillout_form_id, status, is_required, display_order, is_active, created_at, updated_at
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

    pub async fn update_form_template(&self, request: &UpdateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE form_templates
            SET form_name = $3, form_type = $4, fillout_form_id = $5, status = $6, is_required = $7, display_order = $8, updated_at = NOW()
            WHERE id = $2 AND school_id = $1 AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, form_name, form_type, fillout_form_id, status, is_required, display_order, is_active, created_at, updated_at
        "#;

        let stmt = client.prepare(query).await
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = client.query(&stmt, &[&request.school_id, &request.id, &request.form_name, &request.form_type, &request.fillout_form_id, &request.status, &request.is_required, &request.display_order])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update form template: {}", e)))?;

        if rows.is_empty() {
            return Err(AppError::NotFound("Form template not found".to_string()));
        }

        Ok(Self::row_to_form_template(&rows[0]))
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