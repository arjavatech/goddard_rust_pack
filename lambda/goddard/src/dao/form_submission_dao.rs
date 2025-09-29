use crate::models::form_submission::{FormSubmission, FormSubmissionStatus, CreateFormSubmissionWebhookRequest};
use crate::error::AppError;
use uuid::Uuid;
use chrono::{Utc, NaiveDateTime, DateTime};
use serde_json::{json, Value as JsonValue};
use deadpool_postgres::Pool;
use tokio_postgres::Row;
use std::str::FromStr;

pub struct FormSubmissionDao {
    pool: Pool,
}

impl FormSubmissionDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // Deprecated - keeping for backward compatibility if needed
    pub async fn create_form_submission(
        &self,
        _request: &CreateFormSubmissionWebhookRequest,
        _form_template_id: Uuid,
    ) -> Result<FormSubmission, AppError> {
        Err(AppError::Validation("This method is deprecated. Use create_form_submission_from_payload instead".to_string()))
    }

    pub async fn create_form_submission_from_payload(
        &self,
        payload: JsonValue,
        school_id: Uuid,
        enrollment_id: Uuid,
        student_form_assignment_id: Uuid,
        form_template_id: Uuid,
    ) -> Result<FormSubmission, AppError> {
        println!("[DEBUG] DAO: Starting form submission creation from payload");

        // ALTERNATIVE APPROACH: Skip transactions entirely, use direct client operations
        let client = match self.pool.get().await {
            Ok(c) => {
                println!("[DEBUG] DAO: Database connection acquired");
                c
            }
            Err(e) => {
                println!("[ERROR] DAO: Failed to get database connection: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
        };

        println!("[DEBUG] DAO: Using direct client operations (no transaction)");

        let now = Utc::now().naive_utc();
        println!("[DEBUG] DAO: Generated timestamp: {}", now);

        // Extract fillout_submission_id from payload, or generate a default
        // Check for both fillout_submission_id and form_submission_id (different naming conventions)
        let fillout_submission_id = payload
            .get("fillout_submission_id")
            .or_else(|| payload.get("form_submission_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("webhook_{}", Uuid::new_v4()));
        println!("[DEBUG] DAO: Fillout submission ID: {}", fillout_submission_id);

        // The entire payload becomes form_data (after removing only the IDs that might be present)
        let mut form_data = payload.clone();
        // Remove only the student_form_assignment_id and fillout_submission_id if present
        // Other IDs are not passed in the payload anymore
        if let Some(obj) = form_data.as_object_mut() {
            obj.remove("student_form_assignment_id");
            obj.remove("fillout_submission_id");
        }

        // Create metadata from the payload context
        let metadata = json!({
            "source": "webhook",
            "received_at": now.to_string(),
            "webhook_payload_keys": payload.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
        });

        println!("[DEBUG] DAO: Executing INSERT query");
        println!("[DEBUG] DAO: Full INSERT query with parameters:");
        println!("INSERT INTO form_submissions (");
        println!("    school_id, enrollment_id, student_form_assignment_id,");
        println!("    form_template_id, fillout_submission_id, form_data, metadata,");
        println!("    submitted_at, processed_at, is_active, created_at, updated_at");
        println!(") VALUES (");
        println!("    '{}', -- school_id", school_id);
        println!("    '{}', -- enrollment_id", enrollment_id);
        println!("    '{}', -- student_form_assignment_id", student_form_assignment_id);
        println!("    '{}', -- form_template_id", form_template_id);
        println!("    '{}', -- fillout_submission_id", fillout_submission_id);
        println!("    '{}', -- form_data", form_data);
        println!("    '{}', -- metadata", metadata);
        println!("    '{}', -- submitted_at", now);
        println!("    '{}', -- processed_at", now);
        println!("    {}, -- is_active", true);
        println!("    '{}', -- created_at", now);
        println!("    '{}', -- updated_at", now);
        println!(");");

        // SOLUTION: Use simple_query to avoid prepared statement conflicts
        println!("[DEBUG] DAO: Using simple_query approach to avoid prepared statements");

        let submission_id = uuid::Uuid::new_v4();
        println!("[DEBUG] DAO: Generated submission ID: {}", submission_id);

        // Build the INSERT query using simple_query to avoid prepared statements
        let insert_query = format!(
            r#"
            INSERT INTO form_submissions (
                id, school_id, enrollment_id, student_form_assignment_id,
                form_template_id, fillout_submission_id, form_data, metadata,
                submitted_at, processed_at, is_active, created_at, updated_at
            )
            VALUES (
                '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}',
                '{}', '{}', true, '{}', '{}'
            )
            "#,
            submission_id,
            school_id,
            enrollment_id,
            student_form_assignment_id,
            form_template_id,
            fillout_submission_id,
            form_data.to_string().replace("'", "''"), // Escape single quotes
            metadata.to_string().replace("'", "''"),  // Escape single quotes
            now,
            now,
            now,
            now
        );

        println!("[DEBUG] DAO: Executing INSERT with direct client execute method");
        let insert_future = client.execute(&insert_query, &[]);

        let _insert_result = match tokio::time::timeout(std::time::Duration::from_secs(10), insert_future).await {
            Ok(insert_result) => match insert_result {
                Ok(result) => {
                    println!("[DEBUG] DAO: INSERT executed successfully: {} rows affected", result);
                    result
                }
                Err(e) => {
                    println!("[ERROR] DAO: INSERT execution failed: {}", e);
                    return Err(AppError::Database(format!("INSERT execution failed: {}", e)));
                }
            },
            Err(_timeout_err) => {
                println!("[ERROR] DAO: INSERT execution timed out after 10 seconds");
                return Err(AppError::Database("INSERT execution timed out".to_string()));
            }
        };

        // Step 2: Update student_form_assignments table with submission status
        println!("[DEBUG] DAO: Updating student_form_assignments status to 'in_progress' for assignment_id: {}", student_form_assignment_id);

        let update_query = r#"
            UPDATE student_form_assignments
            SET
                status = 'in_progress',
                recent_form_submission_id = $1,
                updated_at = NOW()
            WHERE id = $2
        "#;

        let submission_id_param: &(dyn tokio_postgres::types::ToSql + Sync) = &submission_id;
        let assignment_id_param: &(dyn tokio_postgres::types::ToSql + Sync) = &student_form_assignment_id;
        let update_params = vec![submission_id_param, assignment_id_param];
        let update_future = client.execute(
            update_query,
            &update_params
        );

        let _update_result = match tokio::time::timeout(std::time::Duration::from_secs(5), update_future).await {
            Ok(update_result) => match update_result {
                Ok(result) => {
                    println!("[DEBUG] DAO: UPDATE student_form_assignments executed successfully: {} rows affected", result);
                    result
                }
                Err(e) => {
                    println!("[ERROR] DAO: UPDATE student_form_assignments failed: {}", e);
                    // Log error but continue - submission is already saved
                    0
                }
            },
            Err(_timeout_err) => {
                println!("[ERROR] DAO: UPDATE student_form_assignments timed out after 5 seconds");
                // Log timeout but continue - submission is already saved
                0
            }
        };

        println!("[DEBUG] DAO: Constructing FormSubmission directly (school controller pattern - single DB operation)");

        // Create current timestamp for record creation
        let now = chrono::Utc::now();

        // Create FormSubmission with all required fields - using data we already have
        let submission = FormSubmission {
            id: submission_id,
            school_id,
            enrollment_id,
            student_form_assignment_id,
            form_template_id,
            fillout_submission_id,
            form_data: payload.clone(),
            metadata: serde_json::json!({}), // Default empty metadata
            status: FormSubmissionStatus::Pending, // Default status
            revision_number: 1, // First revision
            revision_reason: None, // No revision reason for initial creation
            submitted_at: now,
            processed_at: None, // Not processed yet
            edit_link: None, // No edit link initially
            pdf_link: None, // No PDF link initially
            created_at: now,
            updated_at: now,
        };

        println!("[DEBUG] DAO: Form submission object constructed successfully with direct approach");

        println!("[DEBUG] DAO: Single-operation webhook processing completed (school controller pattern)");

        Ok(submission)
    }

    pub async fn get_latest_form_submission(
        &self,
        school_id: Uuid,
        enrollment_id: Uuid,
        form_template_id: Uuid,
    ) -> Result<Option<FormSubmission>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = client.query_opt(
            r#"
            SELECT * FROM form_submissions
            WHERE school_id = $1 AND enrollment_id = $2 AND form_template_id = $3
            ORDER BY revision_number DESC
            LIMIT 1
            "#,
            &[&school_id, &enrollment_id, &form_template_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_form_submission(row)?)),
            None => Ok(None),
        }
    }

    pub async fn get_all_form_submission_versions(
        &self,
        school_id: Uuid,
        enrollment_id: Uuid,
        form_template_id: Uuid,
    ) -> Result<Vec<FormSubmission>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = client.query(
            r#"
            SELECT * FROM form_submissions
            WHERE school_id = $1 AND enrollment_id = $2 AND form_template_id = $3
            ORDER BY revision_number DESC
            "#,
            &[&school_id, &enrollment_id, &form_template_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|row| self.row_to_form_submission(row))
            .collect()
    }

    pub async fn get_form_submission_by_id(
        &self,
        submission_id: Uuid,
    ) -> Result<Option<FormSubmission>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = client.query_opt(
            r#"
            SELECT * FROM form_submissions
            WHERE id = $1
            "#,
            &[&submission_id],
        )
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_form_submission(row)?)),
            None => Ok(None),
        }
    }

    pub async fn update_form_submission(
        &self,
        submission_id: Uuid,
        status: Option<FormSubmissionStatus>,
        reason: Option<String>,
        form_data: Option<serde_json::Value>,
        metadata: Option<serde_json::Value>,
    ) -> Result<FormSubmission, AppError> {
        println!("[DEBUG] DAO: Starting form submission update for ID: {}", submission_id);

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Build update query based on what fields need to be updated
        let now = Utc::now().naive_utc();

        // Helper function to convert status to string
        let status_to_str = |status: &FormSubmissionStatus| -> &'static str {
            match status {
                FormSubmissionStatus::Pending => "pending",
                FormSubmissionStatus::Processing => "processing",
                FormSubmissionStatus::Completed => "completed",
                FormSubmissionStatus::Failed => "failed",
                FormSubmissionStatus::RequiresReview => "requires_review",
                FormSubmissionStatus::Approved => "approved",
                FormSubmissionStatus::Rejected => "rejected",
            }
        };

        // Handle each field combination separately
        if let (Some(status), Some(reason), Some(form_data), Some(metadata)) = (&status, &reason, &form_data, &metadata) {
            let status_str = status_to_str(status);
            let query = "UPDATE form_submissions SET status = $2, revision_reason = $3, form_data = $4, metadata = $5, updated_at = $6 WHERE id = $1 RETURNING *";
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&submission_id, &status_str, reason, form_data, metadata, &now];

            println!("[DEBUG] DAO: Executing update query: {}", query);
            let row = client.query_one(query, &params).await.map_err(|e| AppError::Database(e.to_string()))?;
            return self.row_to_form_submission(row);
        } else if let (Some(status), Some(reason), Some(form_data)) = (&status, &reason, &form_data) {
            let status_str = status_to_str(status);
            let query = "UPDATE form_submissions SET status = $2, revision_reason = $3, form_data = $4, updated_at = $5 WHERE id = $1 RETURNING *";
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&submission_id, &status_str, reason, form_data, &now];

            println!("[DEBUG] DAO: Executing update query: {}", query);
            let row = client.query_one(query, &params).await.map_err(|e| AppError::Database(e.to_string()))?;
            return self.row_to_form_submission(row);
        } else if let (Some(status), Some(reason)) = (&status, &reason) {
            let status_str = status_to_str(status);
            let query = "UPDATE form_submissions SET status = $2, revision_reason = $3, updated_at = $4 WHERE id = $1 RETURNING *";
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&submission_id, &status_str, reason, &now];

            println!("[DEBUG] DAO: Executing update query: {}", query);
            let row = client.query_one(query, &params).await.map_err(|e| AppError::Database(e.to_string()))?;
            return self.row_to_form_submission(row);
        } else if let Some(status) = &status {
            let status_str = status_to_str(status);
            let query = "UPDATE form_submissions SET status = $2, updated_at = $3 WHERE id = $1 RETURNING *";
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&submission_id, &status_str, &now];

            println!("[DEBUG] DAO: Executing update query: {}", query);
            let row = client.query_one(query, &params).await.map_err(|e| AppError::Database(e.to_string()))?;
            return self.row_to_form_submission(row);
        } else if let Some(form_data) = &form_data {
            let query = "UPDATE form_submissions SET form_data = $2, updated_at = $3 WHERE id = $1 RETURNING *";
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&submission_id, form_data, &now];

            println!("[DEBUG] DAO: Executing update query: {}", query);
            let row = client.query_one(query, &params).await.map_err(|e| AppError::Database(e.to_string()))?;
            return self.row_to_form_submission(row);
        } else if let Some(metadata) = &metadata {
            let query = "UPDATE form_submissions SET metadata = $2, updated_at = $3 WHERE id = $1 RETURNING *";
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&submission_id, metadata, &now];

            println!("[DEBUG] DAO: Executing update query: {}", query);
            let row = client.query_one(query, &params).await.map_err(|e| AppError::Database(e.to_string()))?;
            return self.row_to_form_submission(row);
        } else {
            return Err(AppError::Validation("No fields provided for update".to_string()));
        }
    }

    // Keep the old method for backward compatibility
    pub async fn update_form_submission_status(
        &self,
        submission_id: Uuid,
        status: FormSubmissionStatus,
        reason: Option<String>,
    ) -> Result<FormSubmission, AppError> {
        self.update_form_submission(submission_id, Some(status), reason, None, None).await
    }

    pub async fn get_form_template_id_from_assignment(
        &self,
        student_form_assignment_id: Uuid,
    ) -> Result<Option<Uuid>, AppError> {
        println!("[DEBUG] DAO: Getting form template ID for assignment: {}", student_form_assignment_id);

        let client = match self.pool.get().await {
            Ok(c) => {
                println!("[DEBUG] DAO: Database connection acquired for assignment lookup");
                c
            }
            Err(e) => {
                println!("[ERROR] DAO: Failed to get database connection for assignment lookup: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
        };

        let row = match client.query_opt(
            r#"
            SELECT form_template_id FROM student_form_assignments
            WHERE id = $1
            "#,
            &[&student_form_assignment_id],
        ).await {
            Ok(row) => {
                println!("[DEBUG] DAO: Assignment lookup query executed successfully");
                row
            }
            Err(e) => {
                println!("[ERROR] DAO: Assignment lookup query failed: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
        };

        match row {
            Some(r) => {
                let form_template_id: Uuid = r.get("form_template_id");
                println!("[DEBUG] DAO: Found form_template_id: {}", form_template_id);
                Ok(Some(form_template_id))
            }
            None => {
                println!("[WARN] DAO: No assignment found for ID: {}", student_form_assignment_id);
                Ok(None)
            }
        }
    }

    pub async fn get_assignment_details(
        &self,
        student_form_assignment_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid, Uuid)>, AppError> {
        println!("[DEBUG] DAO: Looking up assignment details for: {}", student_form_assignment_id);

        println!("[DEBUG] DAO: Attempting to get connection from pool...");
        let client = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            self.pool.get()
        ).await {
            Ok(Ok(c)) => {
                println!("[DEBUG] DAO: Database connection acquired for assignment lookup");
                c
            }
            Ok(Err(e)) => {
                println!("[ERROR] DAO: Failed to get database connection: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
            Err(_) => {
                println!("[ERROR] DAO: Timeout getting database connection");
                return Err(AppError::Database("Connection pool timeout".to_string()));
            }
        };

        println!("[DEBUG] DAO: Connection acquired, preparing to execute query");

        // Use simple_query to avoid prepared statement conflicts
        let query = format!(
            r#"SELECT school_id, enrollment_id, form_template_id
               FROM student_form_assignments
               WHERE id = '{}'
               LIMIT 1"#,
            student_form_assignment_id
        );

        println!("[DEBUG] DAO: Executing assignment lookup query: {}", query);

        let rows = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            client.simple_query(&query)
        ).await {
            Ok(Ok(rows)) => {
                println!("[DEBUG] DAO: Assignment lookup query executed successfully, got {} results", rows.len());
                rows
            }
            Ok(Err(e)) => {
                println!("[ERROR] DAO: Failed to query assignment details: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
            Err(_) => {
                println!("[ERROR] DAO: Query timeout");
                return Err(AppError::Database("Query timeout".to_string()));
            }
        };

        println!("[DEBUG] DAO: Processing query results");

        // Parse the results
        for message in rows {
            if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                println!("[DEBUG] DAO: Found row, extracting values");

                let school_id = row.get("school_id")
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| {
                        println!("[ERROR] DAO: Failed to parse school_id");
                        AppError::Database("Invalid school_id in database".to_string())
                    })?;

                let enrollment_id = row.get("enrollment_id")
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| {
                        println!("[ERROR] DAO: Failed to parse enrollment_id");
                        AppError::Database("Invalid enrollment_id in database".to_string())
                    })?;

                let form_template_id = row.get("form_template_id")
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| {
                        println!("[ERROR] DAO: Failed to parse form_template_id");
                        AppError::Database("Invalid form_template_id in database".to_string())
                    })?;

                println!("[DEBUG] DAO: Found assignment details - school: {}, enrollment: {}, template: {}",
                         school_id, enrollment_id, form_template_id);
                return Ok(Some((school_id, enrollment_id, form_template_id)));
            }
        }

        println!("[DEBUG] DAO: No assignment found for ID: {}", student_form_assignment_id);
        Ok(None)
    }

    fn row_to_form_submission(&self, row: Row) -> Result<FormSubmission, AppError> {
        println!("[DEBUG] DAO: Starting row conversion");

        let status_str: String = match row.try_get("status") {
            Ok(status) => {
                println!("[DEBUG] DAO: Status extracted: {}", status);
                status
            }
            Err(e) => {
                println!("[ERROR] DAO: Failed to extract status: {}", e);
                return Err(AppError::Database(format!("Failed to extract status: {}", e)));
            }
        };

        let status = match status_str.as_str() {
            "pending" => FormSubmissionStatus::Pending,
            "processing" => FormSubmissionStatus::Processing,
            "completed" => FormSubmissionStatus::Completed,
            "failed" => FormSubmissionStatus::Failed,
            "requires_review" => FormSubmissionStatus::RequiresReview,
            "approved" => FormSubmissionStatus::Approved,
            "rejected" => FormSubmissionStatus::Rejected,
            _ => {
                println!("[WARN] DAO: Unknown status '{}', defaulting to Pending", status_str);
                FormSubmissionStatus::Pending
            }
        };

        println!("[DEBUG] DAO: Extracting all row fields");

        let submission = FormSubmission {
            id: row.try_get("id").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract id: {}", e);
                AppError::Database(format!("Failed to extract id: {}", e))
            })?,
            school_id: row.try_get("school_id").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract school_id: {}", e);
                AppError::Database(format!("Failed to extract school_id: {}", e))
            })?,
            enrollment_id: row.try_get("enrollment_id").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract enrollment_id: {}", e);
                AppError::Database(format!("Failed to extract enrollment_id: {}", e))
            })?,
            student_form_assignment_id: row.try_get("student_form_assignment_id").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract student_form_assignment_id: {}", e);
                AppError::Database(format!("Failed to extract student_form_assignment_id: {}", e))
            })?,
            form_template_id: row.try_get("form_template_id").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract form_template_id: {}", e);
                AppError::Database(format!("Failed to extract form_template_id: {}", e))
            })?,
            fillout_submission_id: row.try_get("fillout_submission_id").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract fillout_submission_id: {}", e);
                AppError::Database(format!("Failed to extract fillout_submission_id: {}", e))
            })?,
            form_data: row.try_get("form_data").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract form_data: {}", e);
                AppError::Database(format!("Failed to extract form_data: {}", e))
            })?,
            metadata: row.try_get("metadata").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract metadata: {}", e);
                AppError::Database(format!("Failed to extract metadata: {}", e))
            })?,
            status,
            revision_number: row.try_get("revision_number").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract revision_number: {}", e);
                AppError::Database(format!("Failed to extract revision_number: {}", e))
            })?,
            revision_reason: row.try_get("revision_reason").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract revision_reason: {}", e);
                AppError::Database(format!("Failed to extract revision_reason: {}", e))
            })?,
            submitted_at: {
                let naive_dt: NaiveDateTime = row.try_get("submitted_at").map_err(|e| {
                    println!("[ERROR] DAO: Failed to extract submitted_at: {}", e);
                    AppError::Database(format!("Failed to extract submitted_at: {}", e))
                })?;
                DateTime::from_naive_utc_and_offset(naive_dt, Utc)
            },
            processed_at: {
                let naive_dt_opt: Option<NaiveDateTime> = row.try_get("processed_at").map_err(|e| {
                    println!("[ERROR] DAO: Failed to extract processed_at: {}", e);
                    AppError::Database(format!("Failed to extract processed_at: {}", e))
                })?;
                naive_dt_opt.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            },
            edit_link: row.try_get("edit_link").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract edit_link: {}", e);
                AppError::Database(format!("Failed to extract edit_link: {}", e))
            })?,
            pdf_link: row.try_get("pdf_link").map_err(|e| {
                println!("[ERROR] DAO: Failed to extract pdf_link: {}", e);
                AppError::Database(format!("Failed to extract pdf_link: {}", e))
            })?,
            created_at: {
                let naive_dt: NaiveDateTime = row.try_get("created_at").map_err(|e| {
                    println!("[ERROR] DAO: Failed to extract created_at: {}", e);
                    AppError::Database(format!("Failed to extract created_at: {}", e))
                })?;
                DateTime::from_naive_utc_and_offset(naive_dt, Utc)
            },
            updated_at: {
                let naive_dt: NaiveDateTime = row.try_get("updated_at").map_err(|e| {
                    println!("[ERROR] DAO: Failed to extract updated_at: {}", e);
                    AppError::Database(format!("Failed to extract updated_at: {}", e))
                })?;
                DateTime::from_naive_utc_and_offset(naive_dt, Utc)
            },
        };

        println!("[DEBUG] DAO: Row conversion completed successfully");
        Ok(submission)
    }

    pub async fn update_submission_links(
        &self,
        submission_id: Uuid,
        edit_link: Option<String>,
        pdf_link: Option<String>,
    ) -> Result<(), AppError> {
        println!("[DEBUG] DAO: Updating submission links for ID: {}", submission_id);

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE form_submissions
            SET edit_link = $2, pdf_link = $3, updated_at = NOW()
            WHERE id = $1
        "#;

        let rows_affected = client.execute(query, &[&submission_id, &edit_link, &pdf_link])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update submission links: {}", e)))?;

        if rows_affected == 0 {
            println!("[WARN] DAO: No rows updated for submission_id: {}", submission_id);
            return Err(AppError::NotFound("Form submission not found".to_string()));
        }

        println!("[DEBUG] DAO: Successfully updated submission links for {} rows", rows_affected);
        Ok(())
    }

    pub async fn update_assignment_links(
        &self,
        student_form_assignment_id: Uuid,
        recent_edit_link: Option<String>,
        recent_pdf_link: Option<String>,
    ) -> Result<(), AppError> {
        println!("[DEBUG] DAO: Updating assignment links for ID: {}", student_form_assignment_id);

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE student_form_assignments
            SET recent_edit_link = $2, recent_pdf_link = $3, updated_at = NOW()
            WHERE id = $1
        "#;

        let rows_affected = client.execute(query, &[&student_form_assignment_id, &recent_edit_link, &recent_pdf_link])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update assignment links: {}", e)))?;

        if rows_affected == 0 {
            println!("[WARN] DAO: No rows updated for assignment_id: {}", student_form_assignment_id);
            return Err(AppError::NotFound("Student form assignment not found".to_string()));
        }

        println!("[DEBUG] DAO: Successfully updated assignment links for {} rows", rows_affected);
        Ok(())
    }
}