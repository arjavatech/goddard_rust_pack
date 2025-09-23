use crate::models::form_submission::{FormSubmission, FormSubmissionStatus, CreateFormSubmissionWebhookRequest};
use crate::error::AppError;
use uuid::Uuid;
use chrono::{Utc, NaiveDateTime, DateTime};
use serde_json::{json, Value as JsonValue};
use deadpool_postgres::Pool;
use tokio_postgres::Row;

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

        let id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        println!("[DEBUG] DAO: Generated ID: {}, timestamp: {}", id, now);

        // Extract fillout_submission_id from payload, or generate a default
        let fillout_submission_id = payload
            .get("fillout_submission_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("webhook_{}", id))
            .to_string();
        println!("[DEBUG] DAO: Fillout submission ID: {}", fillout_submission_id);

        // The entire payload becomes form_data (after removing the IDs we extracted)
        let mut form_data = payload.clone();
        // Remove the extracted fields from form_data to keep only actual form fields
        if let Some(obj) = form_data.as_object_mut() {
            obj.remove("school_id");
            obj.remove("enrollment_id");
            obj.remove("student_form_assignment_id");
            obj.remove("form_template_id");
            obj.remove("fillout_submission_id");
        }

        // Create metadata from the payload context
        let metadata = json!({
            "source": "webhook",
            "received_at": now.to_string(),
            "webhook_payload_keys": payload.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
        });

        println!("[DEBUG] DAO: Executing INSERT query");
        let row = match client.query_one(
            r#"
            INSERT INTO form_submissions (
                id, school_id, enrollment_id, student_form_assignment_id,
                form_template_id, fillout_submission_id, form_data, metadata,
                status, revision_number, submitted_at, processed_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
            &[
                &id,
                &school_id,
                &enrollment_id,
                &student_form_assignment_id,
                &form_template_id,
                &fillout_submission_id,
                &form_data,
                &metadata,
                &"completed",
                &1i32, // First version
                &now,
                &Some(now),
                &now,
                &now,
            ],
        ).await {
            Ok(row) => {
                println!("[DEBUG] DAO: INSERT query executed successfully");
                row
            }
            Err(e) => {
                println!("[ERROR] DAO: INSERT query failed: {}", e);
                return Err(AppError::Database(e.to_string()));
            }
        };

        println!("[DEBUG] DAO: Converting row to form submission");
        match self.row_to_form_submission(row) {
            Ok(submission) => {
                println!("[DEBUG] DAO: Row conversion successful");
                Ok(submission)
            }
            Err(e) => {
                println!("[ERROR] DAO: Row conversion failed: {:?}", e);
                Err(e)
            }
        }
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
}