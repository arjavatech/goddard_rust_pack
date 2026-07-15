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

    // Helper method to check if submission exists by fillout_submission_id
    async fn get_submission_by_fillout_id(
        &self,
        fillout_submission_id: &str,
    ) -> Result<Option<FormSubmission>, AppError> {
        println!("[DEBUG] DAO: Checking if submission exists with fillout_submission_id: {}", fillout_submission_id);

        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let query = format!(
            r#"SELECT * FROM form_submissions WHERE fillout_submission_id = '{}' LIMIT 1"#,
            fillout_submission_id.replace("'", "''")
        );

        let rows = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.simple_query(&query)
        ).await {
            Ok(Ok(rows)) => rows,
            Ok(Err(e)) => {
                println!("[ERROR] DAO: Failed to query existing submission: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
            Err(_) => {
                println!("[ERROR] DAO: Query timeout");
                return Err(AppError::Database("Query timeout".to_string()));
            }
        };

        for message in rows {
            if let tokio_postgres::SimpleQueryMessage::Row(_row) = message {
                println!("[DEBUG] DAO: Found existing submission");
                // Re-query using proper method to get typed row
                let row = client.query_one(
                    "SELECT * FROM form_submissions WHERE fillout_submission_id = $1",
                    &[&fillout_submission_id],
                )
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

                return Ok(Some(self.row_to_form_submission(row)?));
            }
        }

        println!("[DEBUG] DAO: No existing submission found");
        Ok(None)
    }

    pub async fn create_form_submission_from_payload(
        &self,
        payload: JsonValue,
        school_id: Uuid,
        enrollment_id: Uuid,
        student_form_assignment_id: Uuid,
        form_template_id: Uuid,
    ) -> Result<(FormSubmission, bool), AppError> {
        println!("[DEBUG] DAO: Starting form submission creation/update from payload using UPSERT");

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

        let now = Utc::now().naive_utc();
        println!("[DEBUG] DAO: Generated timestamp: {}", now);

        // Extract fillout_submission_id from payload — the self-hosted Fillout sends
        // `submission_id`; older keys kept as fallbacks
        let fillout_submission_id = payload
            .get("submission_id")
            .or_else(|| payload.get("fillout_submission_id"))
            .or_else(|| payload.get("form_submission_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("webhook_{}", Uuid::new_v4()));
        println!("[DEBUG] DAO: Fillout submission ID: {}", fillout_submission_id);

        let form_status = payload.get("form_status").and_then(|v| v.as_str()).map(|s| s.to_string());
        let form_id = payload.get("form_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let is_in_progress = form_status
            .as_deref()
            .map(|s| s.trim().eq_ignore_ascii_case("IN_PROGRESS"))
            .unwrap_or(false);

        // Prepare form_data (owner-mapped answers only) and metadata
        let mut form_data = payload.clone();
        if let Some(obj) = form_data.as_object_mut() {
            for key in [
                "student_form_assignment_id",
                "fillout_submission_id",
                "submission_id",
                "form_id",
                "form_status",
                "edit_link",
                "pdf_link",
            ] {
                obj.remove(key);
            }
        }

        // metadata is rebuilt on every upsert, so form_status flips from
        // IN_PROGRESS to COMPLETED when the completion webhook arrives
        let metadata = json!({
            "source": "webhook",
            "received_at": now.to_string(),
            "form_status": form_status,
            "form_id": form_id,
            "webhook_payload_keys": payload.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
        });

        let submission_id = Uuid::new_v4();
        println!("[DEBUG] DAO: Generated submission ID: {}", submission_id);

        // UPSERT query using ON CONFLICT
        // xmax = 0 indicates INSERT, xmax != 0 indicates UPDATE
        let upsert_query = format!(
            r#"
            INSERT INTO form_submissions (
                id, school_id, enrollment_id, student_form_assignment_id,
                form_template_id, fillout_submission_id, form_data, metadata,
                status, revision_number, revision_reason,
                submitted_at, processed_at, is_active, created_at, updated_at
            )
            VALUES (
                '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}',
                'pending', 1, 'Initial submission',
                '{}', '{}', true, NOW(), NOW()
            )
            ON CONFLICT (fillout_submission_id)
            DO UPDATE SET
                form_data = EXCLUDED.form_data,
                metadata = EXCLUDED.metadata,
                revision_number = form_submissions.revision_number + 1,
                revision_reason = 'Webhook update',
                processed_at = NOW(),
                updated_at = NOW()
            RETURNING *, (xmax = 0) AS is_insert
            "#,
            submission_id,
            school_id,
            enrollment_id,
            student_form_assignment_id,
            form_template_id,
            fillout_submission_id.replace("'", "''"),
            form_data.to_string().replace("'", "''"),
            metadata.to_string().replace("'", "''"),
            now,
            now
        );

        println!("[DEBUG] DAO: Executing UPSERT query with ON CONFLICT");

        let row = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.query_one(&upsert_query, &[])
        ).await {
            Ok(Ok(row)) => {
                println!("[DEBUG] DAO: UPSERT executed successfully");
                row
            }
            Ok(Err(e)) => {
                println!("[ERROR] DAO: UPSERT execution failed: {}", e);
                return Err(AppError::Database(format!("UPSERT execution failed: {}", e)));
            }
            Err(_) => {
                println!("[ERROR] DAO: UPSERT execution timed out");
                return Err(AppError::Database("UPSERT execution timed out".to_string()));
            }
        };

        // Check if this was an INSERT or UPDATE
        let is_insert: bool = row.get("is_insert");
        println!("[DEBUG] DAO: Operation type - is_insert: {}", is_insert);

        // Update student_form_assignments on webhook:
        // - COMPLETED (or missing status): set status to 'in_progress' (pending review),
        //   including flipping 'rejected'/'approved' back on resubmission
        // - IN_PROGRESS (partial save): only track the submission, leave status untouched
        let update_query = if is_in_progress {
            println!("[DEBUG] DAO: form_status=IN_PROGRESS — tracking submission without changing assignment status for {}", student_form_assignment_id);
            r#"
            UPDATE student_form_assignments
            SET
                recent_form_submission_id = $1,
                updated_at = NOW()
            WHERE id = $2
        "#
        } else {
            println!("[DEBUG] DAO: Updating student_form_assignments status to 'in_progress' for assignment_id: {}", student_form_assignment_id);
            r#"
            UPDATE student_form_assignments
            SET
                status = 'in_progress',
                recent_form_submission_id = $1,
                updated_at = NOW()
            WHERE id = $2
        "#
        };

        let submission_id_from_row: Uuid = row.get("id");
        let submission_id_param: &(dyn tokio_postgres::types::ToSql + Sync) = &submission_id_from_row;
        let assignment_id_param: &(dyn tokio_postgres::types::ToSql + Sync) = &student_form_assignment_id;
        let update_params = vec![submission_id_param, assignment_id_param];
        let update_future = client.execute(update_query, &update_params);

        match tokio::time::timeout(std::time::Duration::from_secs(5), update_future).await {
            Ok(Ok(result)) => {
                println!("[DEBUG] DAO: UPDATE student_form_assignments executed successfully: {} rows affected", result);
                if is_insert {
                    println!("[DEBUG] DAO: First submission - status set to 'in_progress'");
                } else {
                    println!("[DEBUG] DAO: Resubmission detected - status changed back to 'in_progress'");
                }
            }
            Ok(Err(e)) => {
                println!("[WARN] DAO: UPDATE student_form_assignments failed: {}", e);
            }
            Err(_) => {
                println!("[WARN] DAO: UPDATE student_form_assignments timed out");
            }
        };

        // Convert row to FormSubmission
        let submission = self.row_to_form_submission(row)?;

        println!("[DEBUG] DAO: Form submission {} completed successfully (is_insert: {})",
                 if is_insert { "created" } else { "updated" }, is_insert);

        Ok((submission, is_insert))
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

    pub async fn get_recent_edit_link_by_assignment(
        &self,
        assignment_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = client.query_opt(
            "SELECT recent_edit_link FROM student_form_assignments WHERE id = $1",
            &[&assignment_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row.and_then(|r| r.get::<_, Option<String>>("recent_edit_link")))
    }

    pub async fn get_fillout_form_id_by_assignment(
        &self,
        assignment_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = client.query_opt(
            r#"SELECT ft.fillout_form_id
               FROM student_form_assignments sfa
               JOIN form_templates ft ON ft.id = sfa.form_template_id
               WHERE sfa.id = $1
               LIMIT 1"#,
            &[&assignment_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row.and_then(|r| r.get::<_, Option<String>>("fillout_form_id")))
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

        // COALESCE keeps existing links when a webhook arrives without them
        // (e.g. an in-progress event with no PDF yet)
        let query = r#"
            UPDATE form_submissions
            SET edit_link = COALESCE($2, edit_link),
                pdf_link = COALESCE($3, pdf_link),
                updated_at = NOW()
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

        // COALESCE keeps existing links when a webhook arrives without them
        let query = r#"
            UPDATE student_form_assignments
            SET recent_edit_link = COALESCE($2, recent_edit_link),
                recent_pdf_link = COALESCE($3, recent_pdf_link),
                updated_at = NOW()
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