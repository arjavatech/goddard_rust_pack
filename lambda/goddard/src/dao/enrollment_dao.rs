use deadpool_postgres::{Pool, Client};
use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};
use tokio_postgres::Row;
use std::time::Duration;

use crate::models::parent_details::{ParentDetailsRow};
use crate::models::enrollment::{
    CreatedUser, CreatedChild, CreatedEnrollment, FormTemplate,
    ClassFormOverride, CreatedFormAssignment, ParentWithAuthDetails,
    ParentWithChildren, ChildWithForms, FormStatus,
    EnrollmentChildWithForms, SchoolFormDetails, ClassWiseCount,
    DeactivateParentResponse, ActivateParentResponse
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

    // Create parent in users table
    pub async fn create_parent(&self, parent_id: Uuid, school_id: Uuid, first_name: &str, last_name: &str, email: &str, role: &str) -> ApiResult<CreatedUser> {
        let query = "INSERT INTO users (id, school_id, first_name, last_name, email, role) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, school_id, first_name, last_name, email, role, is_verified, created_at";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let row = client.query_one(query, &[&parent_id, &school_id, &first_name, &last_name, &email, &role]).await
            .map_err(|e| AppError::Database(format!("Failed to create parent: {}", e)))?;

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

    // Single transaction method to create child with explicit parent verification
    pub async fn create_child(&self, parent_id: Uuid, school_id: Uuid, first_name: &str, last_name: &str, birth_date: NaiveDate, gender: &str) -> ApiResult<CreatedChild> {
        println!("[DEBUG] [create_child] Starting SINGLE TRANSACTION child creation with parent_id: {}, school_id: {}", parent_id, school_id);

        use tokio::time::{timeout, Duration};

        println!("[DEBUG] [create_child] Getting database connection with 5s timeout");
        let client_result = timeout(Duration::from_secs(5), self.pool.get()).await;
        let mut client = match client_result {
            Ok(client) => client.map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?,
            Err(_) => return Err(AppError::Database("Timeout getting database connection for child creation".to_string()))
        };
        println!("[DEBUG] [create_child] Got database connection successfully");

        // Single transaction with parent verification to avoid foreign key issues
        println!("[DEBUG] [create_child] Starting transaction with parent verification");
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        // First verify parent exists in transaction
        let parent_check_query = "SELECT id FROM users WHERE id = $1 AND school_id = $2 LIMIT 1";
        println!("[DEBUG] [create_child] Verifying parent exists: {} in school: {}", parent_id, school_id);

        let parent_check_result = timeout(Duration::from_secs(5),
            transaction.query_opt(parent_check_query, &[&parent_id, &school_id])
        ).await;

        let parent_exists = match parent_check_result {
            Ok(result) => {
                match result.map_err(|e| AppError::Database(format!("Failed to verify parent: {}", e)))? {
                    Some(_) => {
                        println!("[DEBUG] [create_child] Parent verified successfully");
                        true
                    },
                    None => {
                        println!("[DEBUG] [create_child] Parent not found in database");
                        false
                    }
                }
            },
            Err(_) => return Err(AppError::Database("Timeout verifying parent exists".to_string()))
        };

        if !parent_exists {
            transaction.rollback().await.ok();
            return Err(AppError::Database(format!("Parent with id {} not found in school {}", parent_id, school_id)));
        }

        // Now insert child in same transaction
        let child_insert_query = "INSERT INTO children (id, parent_id, school_id, first_name, last_name, birth_date, gender, status, is_active, created_at, updated_at) VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'active', true, NOW(), NOW()) RETURNING id, parent_id, school_id, first_name, last_name, birth_date, gender, status, created_at";

        println!("[DEBUG] [create_child] Inserting child with verified parent");
        let child_result = timeout(Duration::from_secs(10),
            transaction.query_one(child_insert_query, &[&parent_id, &school_id, &first_name, &last_name, &birth_date, &gender])
        ).await;

        let row = match child_result {
            Ok(result) => result.map_err(|e| AppError::Database(format!("Failed to create child: {}", e)))?,
            Err(_) => {
                transaction.rollback().await.ok();
                return Err(AppError::Database("Timeout during child creation INSERT query".to_string()));
            }
        };

        // Commit transaction
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit child creation: {}", e)))?;

        println!("[DEBUG] [create_child] Child created successfully in single transaction");

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
        // Use timeout to prevent hanging - school controller pattern consistency
        use tokio::time::{timeout, Duration};

        let query = "INSERT INTO enrollments (id, child_id, school_id, classroom_id, status, application_status, is_active, created_at, updated_at) VALUES (gen_random_uuid(), $1, $2, $3, 'incomplete', NULL, true, NOW(), NOW()) RETURNING id, child_id, school_id, classroom_id, status, application_status, created_at";

        let client_result = timeout(Duration::from_secs(5), self.pool.get()).await;
        let client = match client_result {
            Ok(client) => client.map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?,
            Err(_) => return Err(AppError::Database("Timeout getting database connection for enrollment creation".to_string()))
        };

        let query_result = timeout(Duration::from_secs(10),
            client.query_one(query, &[&child_id, &school_id, &classroom_id])
        ).await;

        let row = match query_result {
            Ok(result) => result.map_err(|e| AppError::Database(format!("Failed to create enrollment: {}", e)))?,
            Err(_) => return Err(AppError::Database("Timeout during enrollment creation INSERT query".to_string()))
        };

        Ok(CreatedEnrollment {
            id: row.get("id"),
            child_id: row.get("child_id"),
            school_id: row.get("school_id"),
            classroom_id: row.get("classroom_id"),
            status: row.get("status"), // Always returns 'pending' due to COALESCE in query
            application_status: row.get("application_status"),
            created_at: row.get("created_at"),
        })
    }

    // Step 7: Get school default forms
    pub async fn get_school_default_forms(&self, school_id: Uuid) -> ApiResult<Vec<FormTemplate>> {
        let query = "SELECT id, form_name, is_required FROM form_templates WHERE school_id = $1 AND is_active = true AND status = 'school_default' ORDER BY form_name ASC"; ;
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get school default forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| FormTemplate {
            id: row.get("id"),
            form_name: row.get("form_name"),
            is_required: row.get::<_, Option<bool>>("is_required").unwrap_or(false),
        }).collect())
    }

    // Step 8: Get classroom form overrides
    pub async fn get_classroom_form_overrides(&self, classroom_id: Uuid, school_id: Uuid) -> ApiResult<Vec<ClassFormOverride>> {
        let query = "SELECT cfo.id, cfo.form_template_id, ft.form_name, cfo.action, cfo.is_required FROM class_form_overrides cfo JOIN form_templates ft ON cfo.form_template_id = ft.id WHERE cfo.classroom_id = $1 AND cfo.school_id = $2 AND cfo.is_active = true AND ft.is_active = true";
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&classroom_id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get classroom form overrides: {}", e)))?;

        Ok(rows.into_iter().map(|row| ClassFormOverride {
            id: row.get("id"),
            form_template_id: row.get("form_template_id"),
            form_name: row.get("form_name"),
            action: row.get("action"),
            is_required: row.get::<_, Option<bool>>("is_required").unwrap_or(false),
        }).collect())
    }

    // Step 9: Create student form assignment (optimized to avoid N+1)
    pub async fn create_student_form_assignment(&self, enrollment_id: Uuid, child_id: Uuid, school_id: Uuid, form_template_id: Uuid, assignment_source: &str, is_required: bool) -> ApiResult<CreatedFormAssignment> {
        let query = "INSERT INTO student_form_assignments (enrollment_id, child_id, school_id, form_template_id, assignment_source, is_required) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, form_template_id, assignment_source, status, is_required, recent_edit_link, recent_pdf_link";
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
            recent_edit_link: row.get("recent_edit_link"),
            recent_pdf_link: row.get("recent_pdf_link"),
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

    // Batch form assignment creation (school controller pattern - single DB operation)
    pub async fn create_student_form_assignments_batch(
        &self,
        enrollment_id: Uuid,
        child_id: Uuid,
        school_id: Uuid,
        final_forms: std::collections::HashMap<Uuid, (crate::models::enrollment::FormTemplate, String)>,
    ) -> ApiResult<Vec<CreatedFormAssignment>> {
        if final_forms.is_empty() {
            return Ok(Vec::new());
        }

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        // Build batch insert query with form names included (avoiding N+1)
        let mut query_values = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_idx = 1;

        let status_incomplete = "incomplete";
        for (form_template_id, (form_template, assignment_source)) in &final_forms {
            query_values.push(format!(
                "(${}::uuid, ${}::uuid, ${}::uuid, ${}::uuid, ${}, ${}::boolean, ${})",
                param_idx, param_idx + 1, param_idx + 2, param_idx + 3, param_idx + 4, param_idx + 5, param_idx + 6
            ));
            params.push(&enrollment_id);
            params.push(&child_id);
            params.push(&school_id);
            params.push(form_template_id);
            params.push(assignment_source);
            params.push(&form_template.is_required);
            params.push(&status_incomplete);
            param_idx += 7;
        }

        let query = format!(
            "INSERT INTO student_form_assignments (enrollment_id, child_id, school_id, form_template_id, assignment_source, is_required, status)
             VALUES {}
             RETURNING id, form_template_id, assignment_source, status, is_required, recent_edit_link, recent_pdf_link",
            query_values.join(", ")
        );

        let rows = client.query(&query, &params).await
            .map_err(|e| AppError::Database(format!("Failed to create student form assignments batch: {}", e)))?;

        // Build result with form names from HashMap (no additional DB queries)
        let mut assigned_forms = Vec::new();
        for row in rows {
            let form_template_id: Uuid = row.get("form_template_id");
            let form_name = final_forms.get(&form_template_id)
                .map(|(template, _)| template.form_name.clone())
                .unwrap_or_else(|| "Unknown Form".to_string());

            assigned_forms.push(CreatedFormAssignment {
                id: row.get("id"),
                form_template_id,
                form_name,
                assignment_source: row.get("assignment_source"),
                status: row.get("status"),
                is_required: row.get("is_required"),
                recent_edit_link: row.get("recent_edit_link"),
                recent_pdf_link: row.get("recent_pdf_link"),
            });
        }

        Ok(assigned_forms)
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

    // New method to get parent details with children and forms
    pub async fn get_parent_details_with_children_and_forms(&self, school_id: Uuid) -> ApiResult<(Vec<ParentWithChildren>, Vec<ParentWithChildren>)> {
        let query = "
            WITH parent_children_forms AS (
                SELECT
                    u.id as parent_id,
                    u.email as parent_email,
                    u.first_name as parent_first_name,
                    u.last_name as parent_last_name,
                    u.is_active as parent_is_active,
                    CASE WHEN au.last_sign_in_at IS NOT NULL THEN 'signed' ELSE 'not signed' END as signed_status,
                    c.id as child_id,
                    CONCAT(c.first_name, ' ', c.last_name) as child_full_name,
                    c.birth_date as child_dob,
                    c.status as child_status,
                    e.id as enrollment_id,
                    cl.id as classroom_id,
                    cl.name as classroom_name,
                    sfa.id as student_form_assignment_id,
                    sfa.form_template_id,
                    ft.form_name,
                    ft.fillout_form_id,
                    COALESCE(sfa.status, 'incomplete') as form_status,
                    COALESCE(sfa.is_required, false) as is_required,
                    sfa.recent_edit_link,
                    sfa.recent_pdf_link,
                    sfa.approved_by,
                    sfa.approved_on
                FROM users u
                LEFT JOIN auth.users au ON au.email = u.email
                INNER JOIN children c ON c.parent_id = u.id
                INNER JOIN enrollments e ON e.child_id = c.id
                INNER JOIN classrooms cl ON cl.id = e.classroom_id
                LEFT JOIN student_form_assignments sfa ON sfa.child_id = c.id
                LEFT JOIN form_templates ft ON ft.id = sfa.form_template_id
                WHERE u.school_id = $1
                    AND u.role = 'Parent'
                ORDER BY u.email, c.id, sfa.form_template_id
            )
            SELECT
                parent_id,
                parent_email,
                parent_first_name,
                parent_last_name,
                parent_is_active,
                signed_status,
                child_id,
                child_full_name,
                child_dob,
                child_status,
                enrollment_id,
                classroom_id,
                classroom_name,
                COALESCE(
                    json_agg(
                        CASE WHEN form_template_id IS NOT NULL THEN
                            json_build_object(
                                'form_id', 'form_' || COALESCE(form_template_id::text, ''),
                                'student_form_assignment_id', student_form_assignment_id,
                                'fillout_form_id',
                                CASE
                                    WHEN fillout_form_id IS NOT NULL AND student_form_assignment_id IS NOT NULL
                                    THEN REPLACE(fillout_form_id, 'xxxxx', student_form_assignment_id::text)
                                    ELSE fillout_form_id
                                END,
                                'form_name', form_name,
                                'status', form_status,
                                'is_required', is_required,
                                'recent_edit_link', recent_edit_link,
                                'recent_pdf_link', recent_pdf_link,
                                'approved_by', approved_by,
                                'approved_on', approved_on
                            )
                        ELSE NULL END
                    ) FILTER (WHERE form_template_id IS NOT NULL),
                    '[]'::json
                ) as forms
            FROM parent_children_forms
            GROUP BY parent_id, parent_email, parent_first_name, parent_last_name, parent_is_active, signed_status,
                     child_id, child_full_name, child_dob, child_status, enrollment_id,
                     classroom_id, classroom_name
            ORDER BY parent_email, child_full_name
        ";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get parent details with children: {}", e)))?;

        // Group results by parent
        let mut parents_map: std::collections::HashMap<Uuid, (ParentWithChildren, bool)> = std::collections::HashMap::new();

        for row in rows {
            let parent_id: Uuid = row.get("parent_id");
            let parent_is_active: bool = row.get("parent_is_active");
            let forms_json: serde_json::Value = row.get("forms");

            // Parse forms from JSON
            let forms: Vec<FormStatus> = serde_json::from_value(forms_json)
                .unwrap_or_else(|_| vec![]);

            let child = ChildWithForms {
                child_id: row.get("child_id"),
                child_full_name: row.get("child_full_name"),
                child_dob: row.get("child_dob"),
                child_status: row.get("child_status"),
                enrollment_id: row.get("enrollment_id"),
                classroom_id: row.get("classroom_id"),
                classroom_name: row.get("classroom_name"),
                forms,
            };

            // Add to parent or create new parent entry
            parents_map.entry(parent_id)
                .and_modify(|(parent, _)| parent.children.push(child.clone()))
                .or_insert_with(|| (ParentWithChildren {
                    parent_id,
                    parent_email: row.get("parent_email"),
                    parent_first_name: row.get("parent_first_name"),
                    parent_last_name: row.get("parent_last_name"),
                    signed_status: row.get("signed_status"),
                    children: vec![child],
                }, parent_is_active));
        }

        // Separate into active and inactive parents
        let mut active_parents: Vec<ParentWithChildren> = Vec::new();
        let mut inactive_parents: Vec<ParentWithChildren> = Vec::new();

        for (parent, is_active) in parents_map.into_values() {
            if is_active {
                active_parents.push(parent);
            } else {
                inactive_parents.push(parent);
            }
        }

        // Sort both lists
        active_parents.sort_by(|a, b| a.parent_email.cmp(&b.parent_email));
        inactive_parents.sort_by(|a, b| a.parent_email.cmp(&b.parent_email));

        Ok((active_parents, inactive_parents))
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
                AND u.is_active = true
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
                c.parent_id as parent_id,
                u.first_name as parent_first_name,
                u.last_name as parent_last_name,
                c.id as child_id,
                c.first_name as child_first_name,
                c.last_name as child_last_name,
                c.status as child_status,
                cl.name as class_name,
                u.email as primary_email,
                COALESCE(e.status, 'pending') as form_status,
                COALESCE(
                    (
                        SELECT jsonb_object_agg(
                            ft.form_name,
                            jsonb_build_object(
                                'status', COALESCE(sfa.status, 'incomplete'),
                                'assigned_at', TO_CHAR(sfa.assigned_at, 'DD-MM-YYYY')
                            )
                        )
                        FROM student_form_assignments sfa
                        INNER JOIN form_templates ft ON sfa.form_template_id = ft.id
                        WHERE sfa.enrollment_id = e.id
                        AND sfa.child_id = c.id
                        AND (sfa.is_active = true OR sfa.is_active IS NULL)
                    ),
                    '{}'::jsonb
                ) as forms,
                NULL as additional_parent_email
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            JOIN users u ON c.parent_id = u.id
            JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE e.school_id = $1
                AND u.is_active = true
            ORDER BY e.created_at DESC
        ";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get school forms: {}", e)))?;

        Ok(rows.into_iter().map(|row| SchoolFormDetails {
            parent_id: row.get("parent_id"),
            parent_first_name: row.get("parent_first_name"),
            parent_last_name: row.get("parent_last_name"),
            child_id: row.get("child_id"),
            child_first_name: row.get("child_first_name"),
            child_last_name: row.get("child_last_name"),
            child_status: row.get("child_status"),
            class_name: row.get("class_name"),
            primary_email: row.get("primary_email"),
            form_status: row.get("form_status"),
            forms: row.get("forms"),
            additional_parent_email: row.get("additional_parent_email"),
        }).collect())
    }

    // Method for getting class-based enrollments (filtered by both school_id and class_id)
    pub async fn get_class_based_enrollments(&self, school_id: Uuid, class_id: Uuid) -> ApiResult<Vec<SchoolFormDetails>> {
        let query = "
            SELECT
                c.parent_id as parent_id,
                u.first_name as parent_first_name,
                u.last_name as parent_last_name,
                c.id as child_id,
                c.first_name as child_first_name,
                c.last_name as child_last_name,
                c.status as child_status,
                cl.name as class_name,
                u.email as primary_email,
                COALESCE(e.status, 'pending') as form_status,
                COALESCE(e.application_status, '{}') as forms,
                NULL as additional_parent_email
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            JOIN users u ON c.parent_id = u.id
            JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE e.school_id = $1
                AND e.classroom_id = $2
                AND u.is_active = true
            ORDER BY e.created_at DESC
        ";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&school_id, &class_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get class-based enrollments: {}", e)))?;

        Ok(rows.into_iter().map(|row| SchoolFormDetails {
            parent_id: row.get("parent_id"),
            parent_first_name: row.get("parent_first_name"),
            parent_last_name: row.get("parent_last_name"),
            child_id: row.get("child_id"),
            child_first_name: row.get("child_first_name"),
            child_last_name: row.get("child_last_name"),
            child_status: row.get("child_status"),
            class_name: row.get("class_name"),
            primary_email: row.get("primary_email"),
            form_status: row.get("form_status"),
            forms: row.get("forms"),
            additional_parent_email: row.get("additional_parent_email"),
        }).collect())
    }

    // Method for getting class-wise child count
    pub async fn get_class_wise_count(&self, school_id: Uuid) -> ApiResult<Vec<ClassWiseCount>> {
        let query = "
            SELECT
                cl.id as class_id,
                cl.name as class_name,
                COUNT(DISTINCT e.id) as count,
                COALESCE(
                    json_object_agg(
                        cfo.form_template_id,
                        ft.form_name
                    ) FILTER (WHERE cfo.form_template_id IS NOT NULL),
                    '{}'::json
                ) as forms
            FROM classrooms cl
            LEFT JOIN enrollments e ON cl.id = e.classroom_id AND e.school_id = cl.school_id
            LEFT JOIN class_form_overrides cfo ON cfo.classroom_id = cl.id AND cfo.school_id = cl.school_id AND cfo.is_active = true
            LEFT JOIN form_templates ft ON ft.id = cfo.form_template_id AND ft.is_active = true
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
            forms: row.get("forms"),
        }).collect())
    }

    // Get parent details by parent ID with children and forms
    pub async fn get_parent_details_by_id(&self, parent_id: Uuid) -> ApiResult<Vec<ParentDetailsRow>> {
        let query = "
            SELECT
                u.id as parent_id,
                u.email as parent_email,
                u.first_name as parent_first_name,
                u.last_name as parent_last_name,
                CASE WHEN au.last_sign_in_at IS NOT NULL THEN 'signed' ELSE 'not signed' END as signed_status,
                c.id as child_id,
                c.first_name as child_first_name,
                c.last_name as child_last_name,
                c.birth_date as child_dob,
                c.status as child_status,
                e.id as enrollment_id,
                cl.id as classroom_id,
                cl.name as classroom_name,
                sfa.id as student_form_assignment_id,
                ft.id as form_template_id,
                ft.form_name,
                ft.fillout_form_id,
                sfa.status,
                sfa.is_required,
                sfa.recent_edit_link,
                sfa.recent_pdf_link,
                sfa.approved_by,
                sfa.approved_on,
                sfa.assigned_at
            FROM users u
            LEFT JOIN auth.users au ON au.email = u.email
            LEFT JOIN children c ON c.parent_id = u.id OR c.secondary_parent_id = u.id
            LEFT JOIN enrollments e ON e.child_id = c.id AND (e.is_active = true OR e.is_active IS NULL)
            LEFT JOIN classrooms cl ON cl.id = e.classroom_id
            LEFT JOIN student_form_assignments sfa ON sfa.enrollment_id = e.id AND (sfa.is_active = true OR sfa.is_active IS NULL)
            LEFT JOIN form_templates ft ON ft.id = sfa.form_template_id
            WHERE u.id = $1
                AND u.role IN ('Parent', 'primary-parent', 'secondary-parent')
                AND (u.is_active = true OR u.is_active IS NULL)
            ORDER BY c.id, sfa.id";

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let rows = client.query(query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get parent details: {}", e)))?;

        if rows.is_empty() {
            return Err(AppError::NotFound(format!("Parent with ID {} not found", parent_id)));
        }

        Ok(rows.into_iter().map(|row| ParentDetailsRow {
            parent_id: row.get("parent_id"),
            parent_email: row.get("parent_email"),
            parent_first_name: row.get("parent_first_name"),
            parent_last_name: row.get("parent_last_name"),
            signed_status: row.get("signed_status"),
            child_id: row.get("child_id"),
            child_first_name: row.get("child_first_name"),
            child_last_name: row.get("child_last_name"),
            child_dob: row.get("child_dob"),
            child_status: row.get("child_status"),
            enrollment_id: row.get("enrollment_id"),
            classroom_id: row.get("classroom_id"),
            classroom_name: row.get("classroom_name"),
            student_form_assignment_id: row.get("student_form_assignment_id"),
            form_template_id: row.get("form_template_id"),
            form_name: row.get("form_name"),
            fillout_form_id: row.get("fillout_form_id"),
            status: row.get("status"),
            is_required: row.get("is_required"),
            recent_edit_link: row.get("recent_edit_link"),
            recent_pdf_link: row.get("recent_pdf_link"),
            approved_by: row.get("approved_by"),
            approved_on: row.get("approved_on"),
            assigned_at: row.get("assigned_at"),
        }).collect())
    }

    // Deactivate parent and all related children and enrollments
    pub async fn deactivate_parent(&self, parent_id: Uuid) -> ApiResult<DeactivateParentResponse> {
        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        // Start transaction
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        // Step 1: Deactivate enrollments for all children of this parent
        let enrollment_query = "
            UPDATE enrollments e
            SET is_active = false, updated_at = NOW()
            FROM children c
            WHERE e.child_id = c.id AND c.parent_id = $1
            RETURNING e.id
        ";

        let enrollment_rows = transaction.query(enrollment_query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to deactivate enrollments: {}", e)))?;

        let deactivated_enrollments_count = enrollment_rows.len();

        // Step 2: Deactivate all children of this parent
        let children_query = "
            UPDATE children
            SET is_active = false, updated_at = NOW()
            WHERE parent_id = $1
            RETURNING id
        ";

        let children_rows = transaction.query(children_query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to deactivate children: {}", e)))?;

        let deactivated_children_count = children_rows.len();

        // Step 3: Deactivate the parent user
        let parent_query = "
            UPDATE users
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND role = 'Parent'
            RETURNING id
        ";

        let parent_rows = transaction.query(parent_query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to deactivate parent: {}", e)))?;

        if parent_rows.is_empty() {
            return Err(AppError::NotFound(format!("Parent with ID {} not found", parent_id)));
        }

        // Commit transaction
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(DeactivateParentResponse {
            parent_id,
            deactivated_children_count,
            deactivated_enrollments_count,
            message: "Parent and related records deactivated successfully".to_string(),
        })
    }

    // Activate parent and all related children and enrollments
    pub async fn activate_parent(&self, parent_id: Uuid) -> ApiResult<ActivateParentResponse> {
        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        // Start transaction
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        // Step 1: Activate enrollments for all children of this parent
        let enrollment_query = "
            UPDATE enrollments e
            SET is_active = true, updated_at = NOW()
            FROM children c
            WHERE e.child_id = c.id AND c.parent_id = $1
            RETURNING e.id
        ";

        let enrollment_rows = transaction.query(enrollment_query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to activate enrollments: {}", e)))?;

        let activated_enrollments_count = enrollment_rows.len();

        // Step 2: Activate all children of this parent
        let children_query = "
            UPDATE children
            SET is_active = true, updated_at = NOW()
            WHERE parent_id = $1
            RETURNING id
        ";

        let children_rows = transaction.query(children_query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to activate children: {}", e)))?;

        let activated_children_count = children_rows.len();

        // Step 3: Activate the parent user
        let parent_query = "
            UPDATE users
            SET is_active = true, updated_at = NOW()
            WHERE id = $1 AND role = 'Parent'
            RETURNING id
        ";

        let parent_rows = transaction.query(parent_query, &[&parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to activate parent: {}", e)))?;

        if parent_rows.is_empty() {
            return Err(AppError::NotFound(format!("Parent with ID {} not found", parent_id)));
        }

        // Commit transaction
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(ActivateParentResponse {
            parent_id,
            activated_children_count,
            activated_enrollments_count,
            message: "Parent and related records activated successfully".to_string(),
        })
    }

    // Update child status (admin only - no validation, accepts any status value)
    pub async fn update_child_status(&self, child_id: Uuid, status: &str) -> ApiResult<crate::models::enrollment::UpdateChildStatusResponse> {
        self.execute_with_connection(|client| async move {
            let query = "
                UPDATE children
                SET status = $1, updated_at = NOW()
                WHERE id = $2
                RETURNING id, status, updated_at
            ";

            let row = client.query_one(query, &[&status, &child_id]).await
                .map_err(|e| AppError::Database(format!("Failed to update child status: {}", e)))?;

            Ok(crate::models::enrollment::UpdateChildStatusResponse {
                child_id: row.get("id"),
                status: row.get("status"),
                updated_at: row.get("updated_at"),
                message: "Child status updated successfully".to_string(),
            })
        }).await
    }
}