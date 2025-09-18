use tokio_postgres::Row;
use deadpool_postgres::Pool;
use uuid::Uuid;
use chrono::NaiveDate;

use crate::models::enrollment::{
    CreatedUser, CreatedChild, CreatedEnrollment, FormTemplate,
    ClassFormOverride, CreatedFormAssignment, EnrollmentChildWithForms, ClassWiseCount
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

    // Helper: Get parent by ID for Section 8.3
    pub async fn get_parent_by_id(&self, parent_id: Uuid, school_id: Uuid) -> ApiResult<CreatedUser> {
        let query = r#"
            SELECT id, school_id, first_name, last_name, email, role, is_verified, created_at
            FROM users
            WHERE id = $1 AND school_id = $2 AND role = 'Parent' AND (is_active = true OR is_active IS NULL)
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client
            .query_one(query, &[&parent_id, &school_id])
            .await
            .map_err(|e| {
                if e.to_string().contains("no rows returned") {
                    AppError::NotFound("Parent not found or does not belong to this school".to_string())
                } else {
                    AppError::Database(format!("Failed to get parent: {}", e))
                }
            })?;

        Ok(Self::row_to_created_user(&row))
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

    // Helper: Get all parents by school_id for Section 8.7
    pub async fn get_parents_by_school(&self, school_id: Uuid) -> ApiResult<Vec<CreatedUser>> {
        let query = r#"
            SELECT id, school_id, first_name, last_name, email, role, is_verified, created_at
            FROM users
            WHERE school_id = $1 AND role = 'Parent' AND (is_active = true OR is_active IS NULL)
            ORDER BY first_name, last_name
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client
            .query(query, &[&school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get parents: {}", e)))?;

        Ok(rows.into_iter().map(|row| Self::row_to_created_user(&row)).collect())
    }

    // 8.6 Get Enrollment Children with Form Assignments
    pub async fn get_enrollment_children_with_forms(&self, school_id: Uuid) -> ApiResult<Vec<EnrollmentChildWithForms>> {
        let query = r#"
            SELECT DISTINCT
                c.id AS child_id,
                c.first_name AS child_first_name,
                c.last_name AS child_last_name,
                cl.name AS class_name,
                u1.email AS primary_email,
                u2.email AS additional_parent_email,
                e.status AS form_status,
                -- Aggregate forms as JSON object with form_template_id as key and form_name as value
                (
                    SELECT jsonb_object_agg(
                        ft.id::text,
                        ft.form_name
                    )
                    FROM student_form_assignments sfa
                    INNER JOIN form_templates ft ON sfa.form_template_id = ft.id
                    WHERE sfa.enrollment_id = e.id
                    AND sfa.child_id = c.id
                    AND (sfa.is_active = true OR sfa.is_active IS NULL)
                    AND (ft.is_active = true OR ft.is_active IS NULL)
                ) AS forms
            FROM enrollments e
            INNER JOIN children c ON e.child_id = c.id
            INNER JOIN classrooms cl ON e.classroom_id = cl.id
            INNER JOIN users u1 ON c.parent_id = u1.id
            LEFT JOIN users u2 ON c.secondary_parent_id = u2.id
            WHERE e.school_id = $1
                AND (e.is_active = true OR e.is_active IS NULL)
                AND (c.is_active = true OR c.is_active IS NULL)
                AND (cl.is_active = true OR cl.is_active IS NULL)
                AND (u1.is_active = true OR u1.is_active IS NULL)
            ORDER BY c.first_name, c.last_name
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client
            .query(query, &[&school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get enrollment children with forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| Self::row_to_enrollment_child_with_forms(&row)).collect())
    }

    fn row_to_enrollment_child_with_forms(row: &Row) -> EnrollmentChildWithForms {
        EnrollmentChildWithForms {
            child_id: row.get("child_id"),
            child_first_name: row.get("child_first_name"),
            child_last_name: row.get("child_last_name"),
            class_name: row.get("class_name"),
            primary_email: row.get("primary_email"),
            form_status: row.get("form_status"),
            forms: row.get::<_, Option<serde_json::Value>>("forms").unwrap_or(serde_json::json!({})),
            additional_parent_email: row.get("additional_parent_email"),
        }
    }
    // 8.5 Get Class-wise Child Count Details
    pub async fn get_class_wise_count(&self, school_id: Uuid) -> ApiResult<Vec<ClassWiseCount>> {
        let query = r#"
            SELECT
                c.id AS class_id,
                c.name AS class_name,
                COUNT(e.id) AS count,
                COALESCE(
                    jsonb_object_agg(
                        ft.id::text,
                        ft.form_name
                    ) FILTER (WHERE ft.id IS NOT NULL),
                    '{}'
                ) AS forms,
                COALESCE(
                    jsonb_agg(
                        DISTINCT ft_default.form_name
                    ) FILTER (WHERE ft_default.form_name IS NOT NULL),
                    '[]'
                )::text AS default_forms
            FROM classrooms c
            LEFT JOIN enrollments e ON c.id = e.classroom_id
                AND (e.is_active = true OR e.is_active IS NULL)
            LEFT JOIN form_templates ft_default ON ft_default.school_id = c.school_id
                AND (ft_default.is_active = true OR ft_default.is_active IS NULL)
            LEFT JOIN class_form_overrides cfo ON c.id = cfo.classroom_id
                AND (cfo.is_active = true OR cfo.is_active IS NULL)
            LEFT JOIN form_templates ft ON cfo.form_template_id = ft.id
                AND (ft.is_active = true OR ft.is_active IS NULL)
            WHERE c.school_id = $1
                AND (c.is_active = true OR c.is_active IS NULL)
            GROUP BY c.id, c.name
            ORDER BY c.id
        "#;

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client
            .query(query, &[&school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get class-wise count: {}", e)))?;

        Ok(rows.into_iter().map(|row| Self::row_to_class_wise_count(&row)).collect())
    }

    fn row_to_class_wise_count(row: &Row) -> ClassWiseCount {
        ClassWiseCount {
            class_id: row.get("class_id"),
            class_name: row.get("class_name"),
            count: row.get("count"),
            forms: row.get::<_, Option<serde_json::Value>>("forms").unwrap_or(serde_json::json!({})),
            default_forms: row.get("default_forms"),
        }
    }
}
