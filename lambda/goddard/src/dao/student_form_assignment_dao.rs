use crate::models::student_form_assignment::{
    StudentFormAssignment, StudentFormAssignmentStatus, CreateStudentFormAssignmentRequest,
    UpdateStudentFormAssignmentRequest
};
use crate::error::AppError;
use uuid::Uuid;
use chrono::{Utc, NaiveDateTime, DateTime};
use deadpool_postgres::Pool;
use tokio_postgres::Row;

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
}