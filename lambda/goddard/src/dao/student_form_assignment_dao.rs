use crate::models::student_form_assignment::{
    StudentFormAssignment, StudentFormAssignmentStatus, CreateStudentFormAssignmentRequest,
    UpdateStudentFormAssignmentRequest, FormAssignment
};
use crate::models::student_form_assignment_review::{
    ReviewStudentFormAssignmentRequest, ReviewStudentFormAssignmentResponse
};
use crate::dao::enrollment_dao::{AssignmentNotificationContext, ReviewNotificationContext};
use crate::error::AppError;
use uuid::Uuid;
use chrono::{Utc, NaiveDateTime, DateTime};
use deadpool_postgres::Pool;
use tokio_postgres::Row;

pub struct CompletedFormForZip {
    pub assignment_id: Uuid,
    pub form_name: String,
    pub recent_pdf_link: String,
}

pub struct StudentFormAssignmentDao {
    pool: Pool,
}

impl StudentFormAssignmentDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create_student_form_assignment(
        &self,
        request: &CreateStudentFormAssignmentRequest,
    ) -> Result<StudentFormAssignment, AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Starting assignment creation");

        let client = match self.pool.get().await {
            Ok(c) => {
                println!("[DEBUG] StudentFormAssignmentDAO: Database connection acquired");
                c
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to get database connection: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
        };

        let id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        let status = request.status.as_ref().unwrap_or(&StudentFormAssignmentStatus::Incomplete);
        let is_required = request.is_required.unwrap_or(true);

        let status_str = match status {
            StudentFormAssignmentStatus::Incomplete => "incomplete",
            StudentFormAssignmentStatus::InProgress => "in_progress",
            StudentFormAssignmentStatus::Completed => "completed",
            StudentFormAssignmentStatus::Approved => "approved",
            StudentFormAssignmentStatus::Rejected => "rejected",
        };

        println!("[DEBUG] StudentFormAssignmentDAO: Executing INSERT query");
        let row = match client.query_one(
            r#"
            INSERT INTO student_form_assignments (
                id, school_id, enrollment_id, child_id, form_template_id,
                assignment_source, status, is_required, assigned_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
            &[
                &id,
                &request.school_id,
                &request.enrollment_id,
                &request.child_id,
                &request.form_template_id,
                &request.assignment_source,
                &status_str,
                &is_required,
                &now,
            ],
        ).await {
            Ok(row) => {
                println!("[DEBUG] StudentFormAssignmentDAO: INSERT query executed successfully");
                row
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentDAO: INSERT query failed: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
        };

        println!("[DEBUG] StudentFormAssignmentDAO: Converting row to assignment");
        self.row_to_student_form_assignment(row)
    }

    pub async fn get_assignments_by_school(
        &self,
        school_id: Uuid,
    ) -> Result<Vec<StudentFormAssignment>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"
            SELECT * FROM student_form_assignments
            WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
            ORDER BY assigned_at DESC
            "#,
            &[&school_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|row| self.row_to_student_form_assignment(row))
            .collect()
    }

    pub async fn update_student_form_assignment(
        &self,
        request: &UpdateStudentFormAssignmentRequest,
    ) -> Result<StudentFormAssignment, AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Starting assignment update for ID: {}", request.id);

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Build dynamic query based on what fields need to be updated
        let now = Utc::now().naive_utc();
        let mut set_clauses = Vec::new();
        let mut param_count = 3; // Starting after id and school_id

        // Always update the updated_at timestamp
        set_clauses.push(format!("updated_at = ${}", param_count));
        param_count += 1;

        if request.enrollment_id.is_some() {
            set_clauses.push(format!("enrollment_id = ${}", param_count));
            param_count += 1;
        }

        if request.child_id.is_some() {
            set_clauses.push(format!("child_id = ${}", param_count));
            param_count += 1;
        }

        if request.form_template_id.is_some() {
            set_clauses.push(format!("form_template_id = ${}", param_count));
            param_count += 1;
        }

        if request.assignment_source.is_some() {
            set_clauses.push(format!("assignment_source = ${}", param_count));
            param_count += 1;
        }

        if request.status.is_some() {
            set_clauses.push(format!("status = ${}", param_count));
            param_count += 1;
        }

        if request.is_required.is_some() {
            set_clauses.push(format!("is_required = ${}", param_count));
            param_count += 1;
        }

        if set_clauses.len() == 1 { // Only updated_at would be set
            return Err(AppError::Validation("No fields provided for update".to_string()));
        }

        let query = format!(
            "UPDATE student_form_assignments SET {} WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL) RETURNING *",
            set_clauses.join(", ")
        );

        println!("[DEBUG] StudentFormAssignmentDAO: Executing update query: {}", query);

        // Build parameters
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![
            &request.id,
            &request.school_id,
            &now,
        ];

        if let Some(enrollment_id) = &request.enrollment_id {
            params.push(enrollment_id);
        }

        if let Some(child_id) = &request.child_id {
            params.push(child_id);
        }

        if let Some(form_template_id) = &request.form_template_id {
            params.push(form_template_id);
        }

        if let Some(assignment_source) = &request.assignment_source {
            params.push(assignment_source);
        }

        let status_str: Option<String> = request.status.as_ref().map(|s| match s {
            StudentFormAssignmentStatus::Incomplete => "incomplete".to_string(),
            StudentFormAssignmentStatus::InProgress => "in_progress".to_string(),
            StudentFormAssignmentStatus::Completed => "completed".to_string(),
            StudentFormAssignmentStatus::Approved => "approved".to_string(),
            StudentFormAssignmentStatus::Rejected => "rejected".to_string(),
        });
        if let Some(status_value) = &status_str {
            params.push(status_value);
        }

        if let Some(is_required) = &request.is_required {
            params.push(is_required);
        }

        let row = client.query_one(&query, &params)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        println!("[DEBUG] StudentFormAssignmentDAO: Update query executed successfully");
        self.row_to_student_form_assignment(row)
    }

    pub async fn delete_student_form_assignment(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
    ) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows_affected = client.execute(
            r#"
            UPDATE student_form_assignments
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND school_id = $2
            "#,
            &[&assignment_id, &school_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Student form assignment not found".to_string()));
        }

        Ok(())
    }

    pub async fn review_student_form_assignment(
        &self,
        request: &ReviewStudentFormAssignmentRequest,
    ) -> Result<ReviewStudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Starting assignment review for ID: {}", request.assignment_id);

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Convert status enum to string for database
        let status_str = match request.status {
            StudentFormAssignmentStatus::Approved => "approved",
            StudentFormAssignmentStatus::Rejected => "rejected",
            _ => return Err(AppError::Validation("Review status must be 'approved' or 'rejected'".to_string())),
        };

        let now = Utc::now().naive_utc();

        println!("[DEBUG] StudentFormAssignmentDAO: Executing review update query with status: {}", status_str);
        let row = client.query_one(
            r#"
            UPDATE student_form_assignments
            SET
                status = $1,
                notes = $2,
                approved_by = $3,
                approved_on = $4,
                updated_at = $4
            WHERE id = $5
            RETURNING
                id, school_id, enrollment_id, child_id, form_template_id,
                assignment_source, status, is_required, assigned_at,
                notes, approved_by, approved_on, updated_at
            "#,
            &[
                &status_str,
                &request.notes,
                &request.approved_by,
                &now,
                &request.assignment_id,
            ],
        )
        .await
        .map_err(|e| {
            println!("[ERROR] StudentFormAssignmentDAO: Review update failed: {}", e);
            AppError::Database(e.to_string())
        })?;

        println!("[DEBUG] StudentFormAssignmentDAO: Review update completed successfully");

        // Get the status from the returned row to confirm the update
        let status_str: String = row.try_get("status")
            .map_err(|e| AppError::Database(format!("Failed to extract status: {}", e)))?;

        // Convert back to enum for response
        let status = match status_str.as_str() {
            "approved" => StudentFormAssignmentStatus::Approved,
            "rejected" => StudentFormAssignmentStatus::Rejected,
            "incomplete" => StudentFormAssignmentStatus::Incomplete,
            "in_progress" => StudentFormAssignmentStatus::InProgress,
            "completed" => StudentFormAssignmentStatus::Completed,
            _ => return Err(AppError::Database(format!("Unknown status value: {}", status_str))),
        };

        Ok(ReviewStudentFormAssignmentResponse {
            id: row.try_get("id")
                .map_err(|e| AppError::Database(format!("Failed to extract id: {}", e)))?,
            school_id: row.try_get("school_id")
                .map_err(|e| AppError::Database(format!("Failed to extract school_id: {}", e)))?,
            enrollment_id: row.try_get("enrollment_id")
                .map_err(|e| AppError::Database(format!("Failed to extract enrollment_id: {}", e)))?,
            child_id: row.try_get("child_id")
                .map_err(|e| AppError::Database(format!("Failed to extract child_id: {}", e)))?,
            form_template_id: row.try_get("form_template_id")
                .map_err(|e| AppError::Database(format!("Failed to extract form_template_id: {}", e)))?,
            assignment_source: row.try_get("assignment_source")
                .map_err(|e| AppError::Database(format!("Failed to extract assignment_source: {}", e)))?,
            status,
            is_required: row.try_get("is_required")
                .map_err(|e| AppError::Database(format!("Failed to extract is_required: {}", e)))?,
            assigned_at: {
                let naive_dt: NaiveDateTime = row.try_get("assigned_at")
                    .map_err(|e| AppError::Database(format!("Failed to extract assigned_at: {}", e)))?;
                DateTime::from_naive_utc_and_offset(naive_dt, Utc)
            },
            notes: row.try_get("notes")
                .map_err(|e| AppError::Database(format!("Failed to extract notes: {}", e)))?,
            approved_by: row.try_get("approved_by")
                .map_err(|e| AppError::Database(format!("Failed to extract approved_by: {}", e)))?,
            approved_on: {
                let naive_dt_opt: Option<NaiveDateTime> = row.try_get("approved_on")
                    .map_err(|e| AppError::Database(format!("Failed to extract approved_on: {}", e)))?;
                naive_dt_opt.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            },
            updated_at: {
                let naive_dt_opt: Option<NaiveDateTime> = row.try_get("updated_at")
                    .map_err(|e| AppError::Database(format!("Failed to extract updated_at: {}", e)))?;
                naive_dt_opt.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            },
        })
    }

    fn row_to_student_form_assignment(&self, row: Row) -> Result<StudentFormAssignment, AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Starting row conversion");

        let status_str: String = match row.try_get("status") {
            Ok(status) => {
                println!("[DEBUG] StudentFormAssignmentDAO: Status extracted: {}", status);
                status
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract status: {}", e);
                return Err(AppError::Database(format!("Failed to extract status: {}", e)));
            }
        };

        let status = match status_str.as_str() {
            "incomplete" => StudentFormAssignmentStatus::Incomplete,
            "in_progress" => StudentFormAssignmentStatus::InProgress,
            "completed" => StudentFormAssignmentStatus::Completed,
            "approved" => StudentFormAssignmentStatus::Approved,
            "rejected" => StudentFormAssignmentStatus::Rejected,
            _ => {
                println!("[WARN] StudentFormAssignmentDAO: Unknown status '{}', defaulting to Incomplete", status_str);
                StudentFormAssignmentStatus::Incomplete
            }
        };

        println!("[DEBUG] StudentFormAssignmentDAO: Extracting all row fields");

        let assignment = StudentFormAssignment {
            id: row.try_get("id").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract id: {}", e);
                AppError::Database(format!("Failed to extract id: {}", e))
            })?,
            school_id: row.try_get("school_id").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract school_id: {}", e);
                AppError::Database(format!("Failed to extract school_id: {}", e))
            })?,
            enrollment_id: row.try_get("enrollment_id").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract enrollment_id: {}", e);
                AppError::Database(format!("Failed to extract enrollment_id: {}", e))
            })?,
            child_id: row.try_get("child_id").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract child_id: {}", e);
                AppError::Database(format!("Failed to extract child_id: {}", e))
            })?,
            form_template_id: row.try_get("form_template_id").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract form_template_id: {}", e);
                AppError::Database(format!("Failed to extract form_template_id: {}", e))
            })?,
            assignment_source: row.try_get("assignment_source").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract assignment_source: {}", e);
                AppError::Database(format!("Failed to extract assignment_source: {}", e))
            })?,
            status,
            is_required: row.try_get("is_required").map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to extract is_required: {}", e);
                AppError::Database(format!("Failed to extract is_required: {}", e))
            })?,
            assigned_at: {
                let naive_dt: NaiveDateTime = row.try_get("assigned_at").map_err(|e| {
                    println!("[ERROR] StudentFormAssignmentDAO: Failed to extract assigned_at: {}", e);
                    AppError::Database(format!("Failed to extract assigned_at: {}", e))
                })?;
                DateTime::from_naive_utc_and_offset(naive_dt, Utc)
            },
            updated_at: {
                let naive_dt_opt: Option<NaiveDateTime> = row.try_get("updated_at").map_err(|e| {
                    println!("[ERROR] StudentFormAssignmentDAO: Failed to extract updated_at: {}", e);
                    AppError::Database(format!("Failed to extract updated_at: {}", e))
                })?;
                naive_dt_opt.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            },
        };

        println!("[DEBUG] StudentFormAssignmentDAO: Row conversion completed successfully");
        Ok(assignment)
    }

    pub async fn get_completed_assignments_for_zip(
        &self,
        enrollment_id: Uuid,
    ) -> Result<Vec<CompletedFormForZip>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"
            SELECT sfa.id, ft.form_name, sfa.recent_pdf_link
            FROM student_form_assignments sfa
            JOIN form_templates ft ON sfa.form_template_id = ft.id
            WHERE sfa.enrollment_id = $1
              AND sfa.status IN ('completed', 'approved')
              AND sfa.recent_pdf_link IS NOT NULL
              AND (sfa.is_active = true OR sfa.is_active IS NULL)
            ORDER BY ft.form_name
            "#,
            &[&enrollment_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let forms = rows.into_iter().map(|row| {
            CompletedFormForZip {
                assignment_id: row.get("id"),
                form_name: row.get("form_name"),
                recent_pdf_link: row.get("recent_pdf_link"),
            }
        }).collect();

        Ok(forms)
    }

    pub async fn get_enrollment_parent_id(
        &self,
        enrollment_id: Uuid,
    ) -> Result<(Uuid, String, String), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = client.query_opt(
            r#"
            SELECT c.parent_id, c.first_name, c.last_name
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            WHERE e.id = $1
            "#,
            &[&enrollment_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        match row {
            Some(row) => {
                let parent_id: Uuid = row.get("parent_id");
                let first_name: String = row.get("first_name");
                let last_name: String = row.get("last_name");
                Ok((parent_id, first_name, last_name))
            }
            None => Err(AppError::NotFound("Enrollment".to_string())),
        }
    }

    // Validate that all form templates are active
    pub async fn validate_form_templates_active(
        &self,
        form_template_ids: &[Uuid],
    ) -> Result<(), AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Validating {} form templates are active", form_template_ids.len());

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Query to check if all form templates exist and are active
        let query = r#"
            SELECT id FROM form_templates
            WHERE id = ANY($1) AND (is_active = true OR is_active IS NULL)
        "#;

        let rows = client.query(query, &[&form_template_ids])
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let active_ids: Vec<Uuid> = rows.iter()
            .map(|row| row.get::<_, Uuid>(0))
            .collect();

        // Check if all requested IDs are active
        for &template_id in form_template_ids {
            if !active_ids.contains(&template_id) {
                println!("[ERROR] StudentFormAssignmentDAO: Form template {} is not active or does not exist", template_id);
                return Err(AppError::Validation(
                    format!("Form template {} is not active or does not exist", template_id)
                ));
            }
        }

        println!("[DEBUG] StudentFormAssignmentDAO: All form templates validated successfully");
        Ok(())
    }

    // Check for duplicate assignments (child_id + form_template_id combination)
    pub async fn check_duplicate_assignments(
        &self,
        school_id: Uuid,
        assignments: &[FormAssignment],
    ) -> Result<(), AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Checking for duplicate assignments");

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // For each assignment, check if active assignment already exists
        for assignment in assignments {
            let query = r#"
                SELECT id FROM student_form_assignments
                WHERE school_id = $1
                  AND child_id = $2
                  AND form_template_id = $3
                  AND (is_active = true OR is_active IS NULL)
            "#;

            let rows = client.query(
                query,
                &[&school_id, &assignment.child_id, &assignment.form_template_id]
            )
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            if !rows.is_empty() {
                println!("[ERROR] StudentFormAssignmentDAO: Duplicate assignment found for child {} and form {}",
                         assignment.child_id, assignment.form_template_id);
                return Err(AppError::Conflict(
                    format!(
                        "Student {} already has form {} assigned",
                        assignment.child_id, assignment.form_template_id
                    )
                ));
            }
        }

        println!("[DEBUG] StudentFormAssignmentDAO: No duplicate assignments found");
        Ok(())
    }

    // Bulk create assignments in a transaction
    pub async fn bulk_create_assignments(
        &self,
        school_id: Uuid,
        assignments: Vec<FormAssignment>,
    ) -> Result<Vec<StudentFormAssignment>, AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Starting bulk creation of {} assignments", assignments.len());

        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Start transaction
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        let mut created_assignments = Vec::new();
        let now = Utc::now().naive_utc();

        for assignment in assignments {
            let id = Uuid::new_v4();
            let is_required = assignment.is_required.unwrap_or(false);
            let status_str = "incomplete"; // Default status
            let assignment_source = "manual"; // Manual assignment source

            println!("[DEBUG] StudentFormAssignmentDAO: Inserting assignment for child {}, form {}",
                     assignment.child_id, assignment.form_template_id);

            let row = transaction.query_one(
                r#"
                INSERT INTO student_form_assignments (
                    id, school_id, enrollment_id, child_id, form_template_id,
                    assignment_source, status, is_required, assigned_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING *
                "#,
                &[
                    &id,
                    &school_id,
                    &assignment.enrollment_id,
                    &assignment.child_id,
                    &assignment.form_template_id,
                    &assignment_source,
                    &status_str,
                    &is_required,
                    &now,
                ],
            )
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to insert assignment: {}", e);
                AppError::Database(e.to_string())
            })?;

            let created = self.row_to_student_form_assignment(row)?;
            created_assignments.push(created);
        }

        // Commit transaction
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit transaction: {}", e)))?;

        println!("[DEBUG] StudentFormAssignmentDAO: Successfully created {} assignments", created_assignments.len());
        Ok(created_assignments)
    }

    /// Assign a form template to all active students in a school
    /// Skips students who already have the form assigned
    pub async fn assign_form_to_school_students(
        &self,
        school_id: Uuid,
        form_template_id: Uuid,
        is_required: bool,
    ) -> Result<(Vec<StudentFormAssignment>, i64, i64), AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Assigning form {} to all active students in school {}", form_template_id, school_id);

        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        // Start transaction
        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        // First, get count of total active students
        let total_count_query = r#"
            SELECT COUNT(DISTINCT e.id) as total
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            WHERE e.school_id = $1
              AND (e.is_active = true OR e.is_active IS NULL)
              AND (c.status = 'active' OR c.status IS NULL)
        "#;

        let total_row = transaction.query_one(total_count_query, &[&school_id])
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to count total students: {}", e);
                AppError::Database(e.to_string())
            })?;

        let total_active_students: i64 = total_row.get("total");
        println!("[DEBUG] StudentFormAssignmentDAO: Total active students in school: {}", total_active_students);

        // Get count of students already assigned
        let already_assigned_query = r#"
            SELECT COUNT(DISTINCT sfa.child_id) as already_assigned
            FROM student_form_assignments sfa
            WHERE sfa.school_id = $1
              AND sfa.form_template_id = $2
              AND (sfa.is_active = true OR sfa.is_active IS NULL)
        "#;

        let already_assigned_row = transaction.query_one(already_assigned_query, &[&school_id, &form_template_id])
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to count already assigned: {}", e);
                AppError::Database(e.to_string())
            })?;

        let students_already_assigned: i64 = already_assigned_row.get("already_assigned");
        println!("[DEBUG] StudentFormAssignmentDAO: Students already assigned: {}", students_already_assigned);

        // Get all active enrollments that don't have this form assigned yet
        let query = r#"
            SELECT DISTINCT
                e.id as enrollment_id,
                e.child_id,
                e.school_id
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            WHERE e.school_id = $1
              AND (e.is_active = true OR e.is_active IS NULL)
              AND (c.status = 'active' OR c.status IS NULL)
              AND NOT EXISTS (
                SELECT 1 FROM student_form_assignments sfa
                WHERE sfa.school_id = $1
                  AND sfa.child_id = e.child_id
                  AND sfa.form_template_id = $2
                  AND (sfa.is_active = true OR sfa.is_active IS NULL)
              )
        "#;

        let rows = transaction.query(query, &[&school_id, &form_template_id])
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to fetch active enrollments: {}", e);
                AppError::Database(e.to_string())
            })?;

        println!("[DEBUG] StudentFormAssignmentDAO: Found {} students to assign", rows.len());

        // Create assignments for each student
        let mut created_assignments = Vec::new();
        let assignment_source = "school_default";
        let status_str = "incomplete";
        let now = Utc::now().naive_utc();

        for row in rows {
            let enrollment_id: Uuid = row.get("enrollment_id");
            let child_id: Uuid = row.get("child_id");
            let row_school_id: Uuid = row.get("school_id");

            let id = Uuid::new_v4();

            let insert_query = r#"
                INSERT INTO student_form_assignments (
                    id, school_id, enrollment_id, child_id, form_template_id,
                    assignment_source, status, is_required, assigned_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING
                    id, school_id, enrollment_id, child_id, form_template_id,
                    assignment_source, status, is_required, assigned_at,
                    updated_at
            "#;

            let created_row = transaction.query_one(
                insert_query,
                &[
                    &id,
                    &row_school_id,
                    &enrollment_id,
                    &child_id,
                    &form_template_id,
                    &assignment_source,
                    &status_str,
                    &is_required,
                    &now,
                ],
            )
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to insert assignment for child {}: {}", child_id, e);
                AppError::Database(e.to_string())
            })?;

            let created = self.row_to_student_form_assignment(created_row)?;
            created_assignments.push(created);
        }

        // Commit transaction
        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit transaction: {}", e)))?;

        println!("[DEBUG] StudentFormAssignmentDAO: Successfully created {} new assignments", created_assignments.len());

        Ok((created_assignments, total_active_students, students_already_assigned))
    }

    /// Assign a form template to all active students in a specific class
    /// Skips students who already have the form assigned
    pub async fn assign_form_to_class_students(
        &self,
        school_id: Uuid,
        class_id: Uuid,
        form_template_id: Uuid,
        is_required: bool,
    ) -> Result<(Vec<StudentFormAssignment>, i64, i64), AppError> {
        println!("[DEBUG] StudentFormAssignmentDAO: Assigning form {} to all active students in class {} of school {}", form_template_id, class_id, school_id);

        let mut client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;

        let transaction = client.transaction().await
            .map_err(|e| AppError::Database(format!("Failed to start transaction: {}", e)))?;

        // Total active students in class
        let total_count_query = r#"
            SELECT COUNT(DISTINCT e.id) as total
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            WHERE e.school_id = $1 AND e.classroom_id = $2
              AND (e.is_active = true OR e.is_active IS NULL)
              AND (c.status = 'active' OR c.status IS NULL)
        "#;

        let total_row = transaction.query_one(total_count_query, &[&school_id, &class_id])
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to count total students in class: {}", e);
                AppError::Database(e.to_string())
            })?;

        let total_active_students: i64 = total_row.get("total");
        println!("[DEBUG] StudentFormAssignmentDAO: Total active students in class: {}", total_active_students);

        // Already assigned in class
        let already_assigned_query = r#"
            SELECT COUNT(DISTINCT sfa.child_id) as already_assigned
            FROM student_form_assignments sfa
            JOIN enrollments e ON sfa.enrollment_id = e.id
            WHERE sfa.school_id = $1 AND e.classroom_id = $2
              AND sfa.form_template_id = $3
              AND (sfa.is_active = true OR sfa.is_active IS NULL)
        "#;

        let already_assigned_row = transaction.query_one(already_assigned_query, &[&school_id, &class_id, &form_template_id])
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to count already assigned in class: {}", e);
                AppError::Database(e.to_string())
            })?;

        let students_already_assigned: i64 = already_assigned_row.get("already_assigned");
        println!("[DEBUG] StudentFormAssignmentDAO: Students already assigned in class: {}", students_already_assigned);

        // Eligible students (not yet assigned) in this class
        let query = r#"
            SELECT DISTINCT
                e.id as enrollment_id,
                e.child_id,
                e.school_id
            FROM enrollments e
            JOIN children c ON e.child_id = c.id
            WHERE e.school_id = $1 AND e.classroom_id = $2
              AND (e.is_active = true OR e.is_active IS NULL)
              AND (c.status = 'active' OR c.status IS NULL)
              AND NOT EXISTS (
                SELECT 1 FROM student_form_assignments sfa
                WHERE sfa.school_id = $1
                  AND sfa.child_id = e.child_id
                  AND sfa.form_template_id = $3
                  AND (sfa.is_active = true OR sfa.is_active IS NULL)
              )
        "#;

        let rows = transaction.query(query, &[&school_id, &class_id, &form_template_id])
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to fetch eligible enrollments in class: {}", e);
                AppError::Database(e.to_string())
            })?;

        println!("[DEBUG] StudentFormAssignmentDAO: Found {} students to assign in class", rows.len());

        let mut created_assignments = Vec::new();
        let assignment_source = "class_override";
        let status_str = "incomplete";
        let now = Utc::now().naive_utc();

        for row in rows {
            let enrollment_id: Uuid = row.get("enrollment_id");
            let child_id: Uuid = row.get("child_id");
            let row_school_id: Uuid = row.get("school_id");

            let id = Uuid::new_v4();

            let insert_query = r#"
                INSERT INTO student_form_assignments (
                    id, school_id, enrollment_id, child_id, form_template_id,
                    assignment_source, status, is_required, assigned_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING
                    id, school_id, enrollment_id, child_id, form_template_id,
                    assignment_source, status, is_required, assigned_at,
                    updated_at
            "#;

            let created_row = transaction.query_one(
                insert_query,
                &[
                    &id,
                    &row_school_id,
                    &enrollment_id,
                    &child_id,
                    &form_template_id,
                    &assignment_source,
                    &status_str,
                    &is_required,
                    &now,
                ],
            )
            .await
            .map_err(|e| {
                println!("[ERROR] StudentFormAssignmentDAO: Failed to insert class assignment for child {}: {}", child_id, e);
                AppError::Database(e.to_string())
            })?;

            let created = self.row_to_student_form_assignment(created_row)?;
            created_assignments.push(created);
        }

        transaction.commit().await
            .map_err(|e| AppError::Database(format!("Failed to commit transaction: {}", e)))?;

        println!("[DEBUG] StudentFormAssignmentDAO: Successfully created {} new class assignments", created_assignments.len());

        Ok((created_assignments, total_active_students, students_already_assigned))
    }

    /// Returns the data needed to render a "Form Assigned" notification email
    /// for a given freshly created assignment. See docs/EMAIL_NOTIFICATIONS.md.
    pub async fn get_assignment_notification_context(
        &self,
        assignment_id: Uuid,
    ) -> Result<AssignmentNotificationContext, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;
        let row = client
            .query_one(
                r#"
                SELECT
                    sfa.school_id AS school_id,
                    p.id AS parent_id,
                    p.first_name AS parent_first_name,
                    p.last_name AS parent_last_name,
                    p.email AS parent_email,
                    sp.id AS secondary_parent_id,
                    sp.email AS secondary_parent_email,
                    c.first_name AS child_first_name,
                    c.last_name AS child_last_name,
                    ft.form_name AS form_name,
                    ft.due_date AS due_date,
                    sfa.is_required AS is_required,
                    s.name AS school_name,
                    cl.name AS classroom_name
                FROM student_form_assignments sfa
                INNER JOIN children c ON c.id = sfa.child_id
                INNER JOIN users p ON p.id = c.parent_id
                LEFT JOIN users sp ON sp.id = c.secondary_parent_id
                INNER JOIN form_templates ft ON ft.id = sfa.form_template_id
                INNER JOIN schools s ON s.id = sfa.school_id
                LEFT JOIN enrollments e ON e.id = sfa.enrollment_id
                LEFT JOIN classrooms cl ON cl.id = e.classroom_id
                WHERE sfa.id = $1
                "#,
                &[&assignment_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch assignment context: {}", e)))?;

        let child_first: String = row.get("child_first_name");
        let child_last: String = row.get("child_last_name");

        Ok(AssignmentNotificationContext {
            parent_id: row.get("parent_id"),
            secondary_parent_id: row.get("secondary_parent_id"),
            school_id: row.get("school_id"),
            parent_first_name: row.get("parent_first_name"),
            parent_last_name: row.get("parent_last_name"),
            parent_email: row.get("parent_email"),
            secondary_parent_email: row.get("secondary_parent_email"),
            child_full_name: format!("{} {}", child_first, child_last),
            form_name: row.get("form_name"),
            school_name: row.get("school_name"),
            classroom_name: row.get("classroom_name"),
            is_required: row.get("is_required"),
            due_date: row.get("due_date"),
        })
    }

    /// Returns the data needed to render a Form Approved / Form Rejected
    /// notification email for a given assignment + reviewer. See
    /// docs/EMAIL_NOTIFICATIONS.md.
    pub async fn get_review_notification_context(
        &self,
        assignment_id: Uuid,
        reviewer_id: Uuid,
    ) -> Result<ReviewNotificationContext, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?;
        let row = client
            .query_one(
                r#"
                SELECT
                    sfa.school_id AS school_id,
                    sfa.child_id AS child_id,
                    p.id AS parent_id,
                    p.first_name AS parent_first_name,
                    p.email AS parent_email,
                    sp.id AS secondary_parent_id,
                    sp.email AS secondary_parent_email,
                    c.first_name AS child_first_name,
                    c.last_name AS child_last_name,
                    ft.form_name AS form_name,
                    reviewer.first_name AS reviewer_first_name,
                    reviewer.last_name AS reviewer_last_name
                FROM student_form_assignments sfa
                INNER JOIN children c ON c.id = sfa.child_id
                INNER JOIN users p ON p.id = c.parent_id
                LEFT JOIN users sp ON sp.id = c.secondary_parent_id
                INNER JOIN form_templates ft ON ft.id = sfa.form_template_id
                LEFT JOIN users reviewer ON reviewer.id = $2
                WHERE sfa.id = $1
                "#,
                &[&assignment_id, &reviewer_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch review context: {}", e)))?;

        let child_first: String = row.get("child_first_name");
        let child_last: String = row.get("child_last_name");

        Ok(ReviewNotificationContext {
            parent_id: row.get("parent_id"),
            secondary_parent_id: row.get("secondary_parent_id"),
            school_id: row.get("school_id"),
            child_id: row.get("child_id"),
            parent_first_name: row.get("parent_first_name"),
            parent_email: row.get("parent_email"),
            secondary_parent_email: row.get("secondary_parent_email"),
            child_full_name: format!("{} {}", child_first, child_last),
            form_name: row.get("form_name"),
            reviewer_first_name: row
                .try_get::<_, Option<String>>("reviewer_first_name")
                .ok()
                .flatten()
                .unwrap_or_else(|| "Goddard School".to_string()),
            reviewer_last_name: row
                .try_get::<_, Option<String>>("reviewer_last_name")
                .ok()
                .flatten()
                .unwrap_or_else(|| "Admin".to_string()),
        })
    }
}