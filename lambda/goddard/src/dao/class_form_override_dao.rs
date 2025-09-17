use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::models::class_form_override::{ClassFormOverride, CreateClassFormOverrideRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct ClassFormOverrideDao {
    pool: PgPool,
}

impl ClassFormOverrideDao {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_class_form_override(&self, request: &CreateClassFormOverrideRequest) -> Result<ClassFormOverride, AppError> {
        // First validate that classroom belongs to school
        let classroom_check = sqlx::query("SELECT id FROM classrooms WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)")
            .bind(request.classroom_id)
            .bind(request.school_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to validate classroom: {}", e)))?;

        if classroom_check.is_none() {
            return Err(AppError::NotFound("Classroom not found or does not belong to this school".to_string()));
        }

        // Validate that form template belongs to school
        let form_template_check = sqlx::query("SELECT id FROM form_templates WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)")
            .bind(request.form_template_id)
            .bind(request.school_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to validate form template: {}", e)))?;

        if form_template_check.is_none() {
            return Err(AppError::NotFound("Form template not found or does not belong to this school".to_string()));
        }

        // Check for existing override to prevent duplicates
        let existing_override = sqlx::query("SELECT id FROM class_form_overrides WHERE school_id = $1 AND classroom_id = $2 AND form_template_id = $3 AND (is_active = true OR is_active IS NULL)")
            .bind(request.school_id)
            .bind(request.classroom_id)
            .bind(request.form_template_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to check for existing override: {}", e)))?;

        if existing_override.is_some() {
            return Err(AppError::Validation("Override already exists for this classroom/form combination".to_string()));
        }

        // Create the override
        let query = r#"
            INSERT INTO class_form_overrides (id, school_id, classroom_id, form_template_id, action, is_required, is_active, created_at)
            VALUES (gen_random_uuid(), $1, $2, $3, null, null, true, NOW())
            RETURNING id, school_id, classroom_id, form_template_id, action, is_required, is_active, created_at
        "#;

        let row = sqlx::query(query)
            .bind(request.school_id)
            .bind(request.classroom_id)
            .bind(request.form_template_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create class form override: {}", e)))?;

        Ok(ClassFormOverride {
            id: row.get("id"),
            school_id: row.get("school_id"),
            classroom_id: row.get("classroom_id"),
            form_template_id: row.get("form_template_id"),
            action: row.get("action"),
            is_required: row.get("is_required"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn delete_class_form_override(&self, override_id: &Uuid) -> Result<ClassFormOverride, AppError> {
        // First get the override details for authorization and response
        let get_query = r#"
            SELECT school_id, classroom_id, form_template_id, created_at
            FROM class_form_overrides
            WHERE id = $1 AND (is_active = true OR is_active IS NULL)
        "#;

        let override_details = sqlx::query(get_query)
            .bind(override_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::NotFound("Class form override not found".to_string()),
                _ => AppError::Database(format!("Failed to fetch override details: {}", e)),
            })?;

        // Soft delete the override
        let delete_query = r#"
            UPDATE class_form_overrides
            SET is_active = false
            WHERE id = $1 AND (is_active = true OR is_active IS NULL)
        "#;

        let result = sqlx::query(delete_query)
            .bind(override_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete class form override: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Class form override not found".to_string()));
        }

        Ok(ClassFormOverride {
            id: *override_id,
            school_id: override_details.get("school_id"),
            classroom_id: override_details.get("classroom_id"),
            form_template_id: override_details.get("form_template_id"),
            action: None,
            is_required: None,
            is_active: false,
            created_at: override_details.get("created_at"),
        })
    }
}