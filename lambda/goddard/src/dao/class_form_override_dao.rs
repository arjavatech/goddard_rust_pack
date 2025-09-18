use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use crate::models::class_form_override::{ClassFormOverride, CreateClassFormOverrideRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct ClassFormOverrideDao {
    pool: Pool,
}

impl ClassFormOverrideDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_class_form_override(row: &Row) -> ClassFormOverride {
        ClassFormOverride {
            id: row.get("id"),
            school_id: row.get("school_id"),
            classroom_id: row.get("classroom_id"),
            form_template_id: row.get("form_template_id"),
            action: row.get("action"),
            is_required: row.get("is_required"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
        }
    }

    pub async fn create_class_form_override(&self, request: &CreateClassFormOverrideRequest) -> Result<ClassFormOverride, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Validate classroom exists
        let classroom_check = client.query(
            "SELECT id FROM classrooms WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)",
            &[&request.classroom_id, &request.school_id]
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to validate classroom: {}", e)))?;

        if classroom_check.is_empty() {
            return Err(AppError::NotFound("Classroom not found".to_string()));
        }

        // Validate form template exists
        let form_template_check = client.query(
            "SELECT id FROM form_templates WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)",
            &[&request.form_template_id, &request.school_id]
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to validate form template: {}", e)))?;

        if form_template_check.is_empty() {
            return Err(AppError::NotFound("Form template not found".to_string()));
        }

        // Check if override already exists
        let existing_override = client.query(
            "SELECT id FROM class_form_overrides WHERE school_id = $1 AND classroom_id = $2 AND form_template_id = $3 AND (is_active = true OR is_active IS NULL)",
            &[&request.school_id, &request.classroom_id, &request.form_template_id]
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to check existing override: {}", e)))?;

        if !existing_override.is_empty() {
            return Err(AppError::Validation("Override already exists for this classroom and form template".to_string()));
        }

        let query = r#"
            INSERT INTO class_form_overrides (id, school_id, classroom_id, form_template_id, is_active, created_at)
            VALUES (gen_random_uuid(), $1, $2, $3, true, NOW())
            RETURNING id, school_id, classroom_id, form_template_id, action, is_required, is_active, created_at
        "#;

        let row = client.query_one(
            query,
            &[&request.school_id, &request.classroom_id, &request.form_template_id]
        )
        .await
        .map_err(|e| AppError::Database(format!("Failed to create class form override: {}", e)))?;

        Ok(Self::row_to_class_form_override(&row))
    }

    pub async fn delete_class_form_override(&self, override_id: &Uuid, school_id: &Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Get override details for verification
        let get_query = r#"
            SELECT id, school_id, classroom_id, form_template_id, action, is_required, is_active, created_at
            FROM class_form_overrides
            WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)
        "#;

        let override_details = client.query(get_query, &[override_id, school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch override details: {}", e)))?;

        if override_details.is_empty() {
            return Err(AppError::NotFound("Class form override not found".to_string()));
        }

        // Delete the override
        let delete_query = r#"
            UPDATE class_form_overrides
            SET is_active = false
            WHERE id = $1 AND school_id = $2
        "#;

        let rows_affected = client.execute(delete_query, &[override_id, school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete class form override: {}", e)))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Class form override not found".to_string()));
        }

        Ok(())
    }
}