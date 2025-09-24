use deadpool_postgres::{Pool, Client};
use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};
use tokio_postgres::Row;
use std::time::Duration;

use crate::models::enrollment::{
    CreatedUser, CreatedChild, CreatedEnrollment, FormTemplate,
    ClassFormOverride, CreatedFormAssignment, ParentWithAuthDetails,
    EnrollmentChildWithForms, SchoolFormDetails, ClassWiseCount
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

    // Helper function to get database connection with timeout (same as school DAO)
    async fn get_connection(&self) -> ApiResult<Client> {
        println!("[EnrollmentDao] Attempting to get database connection with 5s timeout");
        let timeout_duration = Duration::from_secs(5);
        let get_connection = self.pool.get();

        match tokio::time::timeout(timeout_duration, get_connection).await {
            Ok(Ok(client)) => {
                println!("[EnrollmentDao] Database connection acquired successfully");
                Ok(client)
            },
            Ok(Err(e)) => {
                println!("[EnrollmentDao] Failed to get connection from pool: {:?}", e);
                Err(AppError::Database(format!("Failed to get connection from pool: {}", e)))
            },
            Err(_) => {
                println!("[EnrollmentDao] Database connection timeout after 5s");
                Err(AppError::Database("Database connection timeout (5s) - database may be unreachable".to_string()))
            }
        }
    }

    // Execute query with automatic connection cleanup (same as school DAO)
    async fn execute_with_connection<T, F, Fut>(&self, operation: F) -> ApiResult<T>
    where
        F: FnOnce(Client) -> Fut,
        Fut: std::future::Future<Output = ApiResult<T>>,
    {
        let client = self.get_connection().await?;
        let result = operation(client).await;
        // Connection is automatically dropped here, returning to pool
        result
    }

    // Step 1: Verify classroom belongs to school
    pub async fn verify_classroom_belongs_to_school(&self, classroom_id: Uuid, school_id: Uuid) -> ApiResult<bool> {
        self.execute_with_connection(|client| async move {
            let query = "SELECT COUNT(*) as count FROM classrooms WHERE id = $1 AND school_id = $2";
            let row = client.query_one(query, &[&classroom_id, &school_id]).await
                .map_err(|e| AppError::Database(format!("Failed to verify classroom belongs to school: {}", e)))?;
            let count: i64 = row.get("count");
            Ok(count > 0)
        }).await
    }

    // Step 2: Check if parent email already exists for this school
    pub async fn check_email_exists(&self, email: &str, school_id: Uuid) -> ApiResult<bool> {
        let email = email.to_string(); // Clone for move
        self.execute_with_connection(|client| async move {
            let query = "SELECT COUNT(*) as count FROM users WHERE email = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)";
            let row = client.query_one(query, &[&email, &school_id]).await
                .map_err(|e| AppError::Database(format!("Failed to check email existence: {}", e)))?;
            let count: i64 = row.get("count");
            Ok(count > 0)
        }).await
    }

    // Step 4: Get parent by ID (should be created by DB trigger)
    pub async fn get_parent_by_id(&self, parent_id: Uuid, school_id: Uuid) -> ApiResult<CreatedUser> {
        let query = "SELECT id, school_id, first_name, last_name, email, role, is_verified, created_at FROM users WHERE id = $1 AND school_id = $2";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client.query_one(query, &[&parent_id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get parent by id: {}", e)))?;

        Ok(CreatedUser {
            id: row.get("id"),
            school_id: row.get("school_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            role: row.get("role"),
            is_verified: row.get("is_verified"),
            created_at: row.get("created_at"),
        })
    }

    // Step 5: Create child in children table
    pub async fn create_child(&self, parent_id: Uuid, school_id: Uuid, first_name: &str, last_name: &str, birth_date: NaiveDate, gender: &str) -> ApiResult<CreatedChild> {
        let query = "INSERT INTO children (parent_id, school_id, first_name, last_name, birth_date, gender) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, parent_id, school_id, first_name, last_name, birth_date, gender, status, created_at";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client.query_one(query, &[&parent_id, &school_id, &first_name, &last_name, &birth_date, &gender]).await
            .map_err(|e| AppError::Database(format!("Failed to create child: {}", e)))?;

        Ok(CreatedChild {
            id: row.get("id"),
            parent_id: row.get("parent_id"),
            school_id: row.get("school_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            birth_date: row.get("birth_date"),
            gender: row.get("gender"),
            status: row.get("status"),
            created_at: row.get("created_at"),
        })
    }

    // Step 6: Create enrollment
    pub async fn create_enrollment(&self, child_id: Uuid, school_id: Uuid, classroom_id: Uuid) -> ApiResult<CreatedEnrollment> {
        let query = "INSERT INTO enrollments (child_id, school_id, classroom_id) VALUES ($1, $2, $3) RETURNING id, child_id, school_id, classroom_id, status, application_status, created_at";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client.query_one(query, &[&child_id, &school_id, &classroom_id]).await
            .map_err(|e| AppError::Database(format!("Failed to create enrollment: {}", e)))?;

        Ok(CreatedEnrollment {
            id: row.get("id"),
            child_id: row.get("child_id"),
            school_id: row.get("school_id"),
            classroom_id: row.get("classroom_id"),
            status: row.get("status"),
            application_status: row.get("application_status"),
            created_at: row.get("created_at"),
        })
    }

    // Step 7: Get school default forms
    pub async fn get_school_default_forms(&self, school_id: Uuid) -> ApiResult<Vec<FormTemplate>> {
        let query = "SELECT id, form_name, is_required FROM form_templates WHERE school_id = $1 AND is_default = true AND is_active = true";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get school default forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| FormTemplate {
            id: row.get("id"),
            form_name: row.get("form_name"),
            is_required: row.get("is_required"),
        }).collect())
    }

    // Step 8: Get classroom form overrides
    pub async fn get_classroom_form_overrides(&self, classroom_id: Uuid, school_id: Uuid) -> ApiResult<Vec<ClassFormOverride>> {
        let query = "SELECT form_template_id, form_name, action, is_required FROM class_form_overrides WHERE classroom_id = $1 AND school_id = $2 AND is_active = true";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&classroom_id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get classroom form overrides: {}", e)))?;

        Ok(rows.into_iter().map(|row| ClassFormOverride {
            id: Uuid::new_v4(), // Placeholder since this field doesn't exist in the query
            form_template_id: row.get("form_template_id"),
            form_name: row.get("form_name"),
            action: row.get("action"),
            is_required: row.get("is_required"),
        }).collect())
    }

    // Step 9: Create student form assignment (optimized to avoid N+1)
    pub async fn create_student_form_assignment(&self, enrollment_id: Uuid, child_id: Uuid, school_id: Uuid, form_template_id: Uuid, assignment_source: &str, is_required: bool) -> ApiResult<CreatedFormAssignment> {
        let query = "INSERT INTO student_form_assignments (enrollment_id, child_id, school_id, form_template_id, assignment_source, is_required) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, form_template_id, assignment_source, status, is_required";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client.query_one(query, &[&enrollment_id, &child_id, &school_id, &form_template_id, &assignment_source, &is_required]).await
            .map_err(|e| AppError::Database(format!("Failed to create student form assignment: {}", e)))?;

        // Get form name separately to avoid N+1 in the main query
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

    // Helper method to get form name (used in create_student_form_assignment)
    async fn get_form_name(&self, form_template_id: Uuid) -> ApiResult<String> {
        let query = "SELECT form_name FROM form_templates WHERE id = $1";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client.query_one(query, &[&form_template_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get form name: {}", e)))?;

        Ok(row.get("form_name"))
    }

    // Additional method for getting parents by school (used in get_parent_details_by_school)
    pub async fn get_parents_by_school(&self, school_id: Uuid) -> ApiResult<Vec<CreatedUser>> {
        let query = "SELECT id, school_id, first_name, last_name, email, role, is_verified, created_at FROM users WHERE school_id = $1 AND role = 'Parent' AND (is_active = true OR is_active IS NULL) ORDER BY created_at DESC";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get parents by school: {}", e)))?;

        Ok(rows.into_iter().map(|row| CreatedUser {
            id: row.get("id"),
            school_id: row.get("school_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            role: row.get("role"),
            is_verified: row.get("is_verified"),
            created_at: row.get("created_at"),
        }).collect())
    }

    // Method for getting enrollment children with form assignments
    pub async fn get_enrollment_children_with_forms(&self, school_id: Uuid) -> ApiResult<Vec<EnrollmentChildWithForms>> {
        let query = "
            SELECT
                c.id as child_id,
                c.first_name as child_first_name,
                c.last_name as child_last_name,
                cl.name as class_name,
                u.email as primary_email,
                'active' as form_status,
                '{}' as forms,
                NULL as additional_parent_email
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            JOIN users u ON c.parent_id = u.id
            JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE e.school_id = $1
            ORDER BY e.created_at DESC
        ";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get enrollment children with forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| EnrollmentChildWithForms {
            child_id: row.get("child_id"),
            child_first_name: row.get("child_first_name"),
            child_last_name: row.get("child_last_name"),
            class_name: row.get("class_name"),
            primary_email: row.get("primary_email"),
            form_status: row.get("form_status"),
            forms: serde_json::from_str(&row.get::<_, String>("forms")).unwrap_or_default(),
            additional_parent_email: row.get("additional_parent_email"),
        }).collect())
    }

    // Method for getting school forms (enrollment form details)
    pub async fn get_school_forms(&self, school_id: Uuid) -> ApiResult<Vec<SchoolFormDetails>> {
        let query = "
            SELECT
                c.id as child_id,
                c.first_name as child_first_name,
                c.last_name as child_last_name,
                cl.name as class_name,
                u.email as primary_email,
                'active' as form_status,
                '{}' as forms,
                NULL as additional_parent_email
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            JOIN users u ON c.parent_id = u.id
            JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE e.school_id = $1
            ORDER BY e.created_at DESC
        ";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get school forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| SchoolFormDetails {
            child_id: row.get("child_id"),
            child_first_name: row.get("child_first_name"),
            child_last_name: row.get("child_last_name"),
            class_name: row.get("class_name"),
            primary_email: row.get("primary_email"),
            form_status: row.get("form_status"),
            forms: serde_json::from_str(&row.get::<_, String>("forms")).unwrap_or_default(),
            additional_parent_email: row.get("additional_parent_email"),
        }).collect())
    }

    // Method for getting class-wise child count
    pub async fn get_class_wise_count(&self, school_id: Uuid) -> ApiResult<Vec<ClassWiseCount>> {
        let query = "
            SELECT
                cl.id as class_id,
                cl.name as class_name,
                COUNT(e.id) as count,
                '{}' as forms,
                'default forms' as default_forms
            FROM classrooms cl
            LEFT JOIN enrollments e ON cl.id = e.classroom_id AND e.school_id = cl.school_id
            WHERE cl.school_id = $1 AND cl.is_active = true
            GROUP BY cl.id, cl.name
            ORDER BY cl.name
        ";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get class-wise count: {}", e)))?;

        Ok(rows.into_iter().map(|row| ClassWiseCount {
            class_id: row.get("class_id"),
            class_name: row.get("class_name"),
            count: row.get("count"),
            forms: serde_json::from_str(&row.get::<_, String>("forms")).unwrap_or_default(),
            default_forms: row.get("default_forms"),
        }).collect())
    }
}