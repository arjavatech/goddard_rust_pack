use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::models::form_template::{FormTemplate, CreateFormTemplateRequest, UpdateFormTemplateRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct FormTemplateDao {
    pool: PgPool,
}

impl FormTemplateDao {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_form_template(&self, request: &CreateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        let query = r#"
            INSERT INTO form_templates (id, school_id, form_name, form_type, fillout_form_id, status, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, 'school_form', $3, 'school_default', true, NOW(), NOW())
            RETURNING id, school_id, form_name, form_type, fillout_form_id, fillout_form_url,
                     status, is_required, display_order, is_active, created_at, updated_at
        "#;

        let row = sqlx::query(query)
            .bind(request.school_id)
            .bind(&request.form_name)
            .bind(&request.fillout_form_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create form template: {}", e)))?;

        Ok(FormTemplate {
            id: row.get("id"),
            school_id: row.get("school_id"),
            form_name: row.get("form_name"),
            form_type: row.get("form_type"),
            fillout_form_id: row.get("fillout_form_id"),
            fillout_form_url: row.get("fillout_form_url"),
            status: row.get("status"),
            is_required: row.get("is_required"),
            display_order: row.get("display_order"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_form_templates_by_school(&self, school_id: &Uuid) -> Result<Vec<FormTemplate>, AppError> {
        let query = r#"
            SELECT id, school_id, form_name, form_type, fillout_form_id, fillout_form_url,
                   status, is_required, display_order, is_active, created_at, updated_at
            FROM form_templates
            WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
            ORDER BY display_order ASC, created_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(school_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch form templates: {}", e)))?;

        let form_templates = rows
            .into_iter()
            .map(|row| {
                FormTemplate {
                    id: row.get("id"),
                    school_id: row.get("school_id"),
                    form_name: row.get("form_name"),
                    form_type: row.get("form_type"),
                    fillout_form_id: row.get("fillout_form_id"),
                    fillout_form_url: row.get("fillout_form_url"),
                    status: row.get("status"),
                    is_required: row.get("is_required"),
                    display_order: row.get("display_order"),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }
            })
            .collect();

        Ok(form_templates)
    }

    pub async fn update_form_template(&self, request: &UpdateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        let query = r#"
            UPDATE form_templates
            SET form_name = $3,
                form_type = $4,
                fillout_form_id = $5,
                fillout_form_url = $6,
                status = $7,
                is_required = $8,
                display_order = $9,
                updated_at = NOW()
            WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, form_name, form_type, fillout_form_id, fillout_form_url,
                     status, is_required, display_order, is_active, created_at, updated_at
        "#;

        let row = sqlx::query(query)
            .bind(request.id)
            .bind(request.school_id)
            .bind(&request.form_name)
            .bind(&request.form_type)
            .bind(&request.fillout_form_id)
            .bind(&request.fillout_form_url)
            .bind(&request.status)
            .bind(request.is_required)
            .bind(request.display_order)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::NotFound("Form template not found".to_string()),
                _ => AppError::Database(format!("Failed to update form template: {}", e)),
            })?;

        Ok(FormTemplate {
            id: row.get("id"),
            school_id: row.get("school_id"),
            form_name: row.get("form_name"),
            form_type: row.get("form_type"),
            fillout_form_id: row.get("fillout_form_id"),
            fillout_form_url: row.get("fillout_form_url"),
            status: row.get("status"),
            is_required: row.get("is_required"),
            display_order: row.get("display_order"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn delete_form_template(&self, form_id: &Uuid, school_id: &Uuid) -> Result<(), AppError> {
        let query = r#"
            UPDATE form_templates
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND school_id = $2
        "#;

        let result = sqlx::query(query)
            .bind(form_id)
            .bind(school_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete form template: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Form template not found".to_string()));
        }

        Ok(())
    }
}