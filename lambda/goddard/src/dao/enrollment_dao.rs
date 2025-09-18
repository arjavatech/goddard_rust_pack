use tokio_postgres::Row;
use deadpool_postgres::Pool;
use uuid::Uuid;
use chrono::NaiveDate;

use crate::models::enrollment::{
    CreatedUser, CreatedChild, CreatedEnrollment, FormTemplate,
    ClassFormOverride, CreatedFormAssignment
};
use crate::error::AppError;

type ApiResult<T> = Result<T, AppError>;

pub struct EnrollmentDao {
    pool: Pool,
}

impl EnrollmentDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // Step 1: Create user in users table with auth_user_id
    pub async fn create_user(
        &self,
        auth_user_id: Uuid,
        school_id: Uuid,
        first_name: &str,
        last_name: &str,
        email: &str,
        role: &str,
    ) -> ApiResult<CreatedUser> {
        let query = r#"
            INSERT INTO users (id, school_id, first_name, last_name, email, role, is_verified, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, false, true, NOW(), NOW())
            RETURNING id, school_id, first_name, last_name, email, role, is_verified, created_at
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&auth_user_id, &school_id, &first_name, &last_name, &email, &role])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create user: {}", e)))?;

        Ok(Self::row_to_created_user(&row))
    }

    // Step 2: Create child in children table
    pub async fn create_child(
        &self,
        parent_id: Uuid,
        school_id: Uuid,
        first_name: &str,
        last_name: &str,
        birth_date: NaiveDate,
        gender: &str,
    ) -> ApiResult<CreatedChild> {
        let query = r#"
            INSERT INTO children (id, parent_id, school_id, first_name, last_name, birth_date, gender, status, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'active', true, NOW(), NOW())
            RETURNING id, parent_id, school_id, first_name, last_name, birth_date, gender, status, created_at
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&parent_id, &school_id, &first_name, &last_name, &birth_date, &gender])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create child: {}", e)))?;

        Ok(Self::row_to_created_child(&row))
    }

    // Step 3: Create enrollment in enrollments table
    pub async fn create_enrollment(
        &self,
        child_id: Uuid,
        school_id: Uuid,
        classroom_id: Uuid,
    ) -> ApiResult<CreatedEnrollment> {
        let query = r#"
            INSERT INTO enrollments (id, child_id, school_id, classroom_id, status, application_status, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, 'incomplete', NULL, true, NOW(), NOW())
            RETURNING id, child_id, school_id, classroom_id, status, application_status, created_at
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&child_id, &school_id, &classroom_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create enrollment: {}", e)))?;

        Ok(Self::row_to_created_enrollment(&row))
    }

    // Step 4: Get school default forms from form_templates
    pub async fn get_school_default_forms(&self, school_id: Uuid) -> ApiResult<Vec<FormTemplate>> {
        let query = r#"
            SELECT id, form_name, is_required
            FROM form_templates
            WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
            ORDER BY form_name
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client
            .query(query, &[&school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get school default forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| Self::row_to_form_template(&row)).collect())
    }

    // Step 5: Get classroom form overrides from class_form_overrides
    pub async fn get_classroom_form_overrides(&self, classroom_id: Uuid, school_id: Uuid) -> ApiResult<Vec<ClassFormOverride>> {
        let query = r#"
            SELECT
                cfo.id,
                cfo.form_template_id,
                ft.form_name,
                cfo.action,
                COALESCE(cfo.is_required, ft.is_required) as is_required
            FROM class_form_overrides cfo
            JOIN form_templates ft ON cfo.form_template_id = ft.id
            WHERE cfo.classroom_id = $1 AND cfo.school_id = $2
            AND (cfo.is_active = true OR cfo.is_active IS NULL)
            AND (ft.is_active = true OR ft.is_active IS NULL)
            ORDER BY ft.form_name
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client
            .query(query, &[&classroom_id, &school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get classroom form overrides: {}", e)))?;

        Ok(rows.into_iter().map(|row| Self::row_to_class_form_override(&row)).collect())
    }

    // Step 6: Create student form assignment
    pub async fn create_student_form_assignment(
        &self,
        enrollment_id: Uuid,
        child_id: Uuid,
        school_id: Uuid,
        form_template_id: Uuid,
        assignment_source: &str,
        is_required: bool,
    ) -> ApiResult<CreatedFormAssignment> {
        let query = r#"
            INSERT INTO student_form_assignments (
                id, enrollment_id, child_id, school_id, form_template_id,
                assignment_source, status, is_required, is_active, created_at, updated_at
            )
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, 'incomplete', $6, true, NOW(), NOW())
            RETURNING id, enrollment_id, child_id, school_id, form_template_id, assignment_source, status, is_required
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&enrollment_id, &child_id, &school_id, &form_template_id, &assignment_source, &is_required])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create form assignment: {}", e)))?;

        // Get form name
        let form_name = self.get_form_name(form_template_id).await?;

        Ok(CreatedFormAssignment {
            id: row.get("id"),
            form_template_id: row.get("form_template_id"),
            form_name,
            assignment_source: row.get("assignment_source"),
            status: row.get("status"),
            is_required: row.get("is_required"),
        })
    }

    // Helper: Get form name by template id
    async fn get_form_name(&self, form_template_id: Uuid) -> ApiResult<String> {
        let query = "SELECT form_name FROM form_templates WHERE id = $1";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&form_template_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get form name: {}", e)))?;

        Ok(row.get("form_name"))
    }

    // Helper: Check if email already exists for school
    pub async fn check_email_exists(&self, email: &str, school_id: Uuid) -> ApiResult<bool> {
        let query = "SELECT COUNT(*) as count FROM users WHERE email = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&email, &school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to check email existence: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    // Helper: Verify classroom belongs to school
    pub async fn verify_classroom_belongs_to_school(&self, classroom_id: Uuid, school_id: Uuid) -> ApiResult<bool> {
        let query = "SELECT COUNT(*) as count FROM classrooms WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&classroom_id, &school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to verify classroom: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    // Row mappers
    fn row_to_created_user(row: &Row) -> CreatedUser {
        CreatedUser {
            id: row.get("id"),
            school_id: row.get("school_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            role: row.get("role"),
            is_verified: row.get("is_verified"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_created_child(row: &Row) -> CreatedChild {
        CreatedChild {
            id: row.get("id"),
            parent_id: row.get("parent_id"),
            school_id: row.get("school_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            birth_date: row.get("birth_date"),
            gender: row.get("gender"),
            status: row.get("status"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_created_enrollment(row: &Row) -> CreatedEnrollment {
        CreatedEnrollment {
            id: row.get("id"),
            child_id: row.get("child_id"),
            school_id: row.get("school_id"),
            classroom_id: row.get("classroom_id"),
            status: row.get("status"),
            application_status: row.get("application_status"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_form_template(row: &Row) -> FormTemplate {
        FormTemplate {
            id: row.get("id"),
            form_name: row.get("form_name"),
            is_required: row.get("is_required"),
        }
    }

    fn row_to_class_form_override(row: &Row) -> ClassFormOverride {
        ClassFormOverride {
            id: row.get("id"),
            form_template_id: row.get("form_template_id"),
            form_name: row.get("form_name"),
            action: row.get("action"),
            is_required: row.get("is_required"),
        }
    }
}