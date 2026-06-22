use crate::dao::{FormSubmissionDao, StudentFormAssignmentDao};
use crate::models::form_submission::{
    CreateFormSubmissionWebhookRequest, FormSubmission, FormSubmissionResponse,
    FormSubmissionStatus, FormSubmissionVersionResponse, UpdateFormSubmissionStatusRequest,
};
use crate::models::notification::{notification_type, CreateNotification};
use crate::services::fillout_service::FilloutService;
use crate::services::NotificationService;
use crate::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

pub struct FormSubmissionService {
    dao: FormSubmissionDao,
    fillout_service: Option<FilloutService>,
    notification_service: Arc<NotificationService>,
    assignment_dao: StudentFormAssignmentDao,
}

impl FormSubmissionService {
    pub fn new(
        dao: FormSubmissionDao,
        notification_service: Arc<NotificationService>,
        assignment_dao: StudentFormAssignmentDao,
    ) -> Self {
        Self {
            dao,
            fillout_service: None,
            notification_service,
            assignment_dao,
        }
    }

    pub fn new_with_fillout(
        dao: FormSubmissionDao,
        fillout_service: FilloutService,
        notification_service: Arc<NotificationService>,
        assignment_dao: StudentFormAssignmentDao,
    ) -> Self {
        Self {
            dao,
            fillout_service: Some(fillout_service),
            notification_service,
            assignment_dao,
        }
    }

    pub async fn create_form_submission_from_webhook(
        &self,
        request: CreateFormSubmissionWebhookRequest,
    ) -> Result<FormSubmissionResponse, AppError> {
        println!("[DEBUG] Service: Starting webhook processing");

        // Handle both nested payload (for local testing) and flat payload (for production webhook)
        let actual_payload = if let Some(nested) = request.payload.get("payload") {
            println!("[DEBUG] Service: Using nested payload structure");
            nested
        } else {
            println!("[DEBUG] Service: Using flat payload structure");
            &request.payload
        };

        // Extract student_form_assignment_id (this is the only required field)
        println!("[DEBUG] Service: Looking for student_form_assignment_id in payload: {:?}", actual_payload);
        let student_form_assignment_id = actual_payload.get("student_form_assignment_id")
            .and_then(|v| {
                println!("[DEBUG] Service: Found student_form_assignment_id value: {:?}", v);
                v.as_str()
            })
            .and_then(|s| {
                println!("[DEBUG] Service: Parsing UUID from string: {}", s);
                Uuid::parse_str(s).ok()
            })
            .ok_or_else(|| {
                println!("[ERROR] Service: Failed to extract student_form_assignment_id from payload");
                AppError::Validation("Missing or invalid student_form_assignment_id in payload".to_string())
            })?;

        println!("[DEBUG] Service: Extracted student_form_assignment_id: {}", student_form_assignment_id);

        // Query the student_form_assignments table to get school_id, enrollment_id, and form_template_id
        let (school_id, enrollment_id, form_template_id) = match self.dao.get_assignment_details(student_form_assignment_id).await {
            Ok(Some(details)) => {
                println!("[DEBUG] Service: Found assignment details - school: {}, enrollment: {}, template: {}",
                         details.0, details.1, details.2);
                details
            }
            Ok(None) => {
                println!("[ERROR] Service: Student form assignment not found: {}", student_form_assignment_id);
                return Err(AppError::NotFound("Student form assignment not found".to_string()));
            }
            Err(e) => {
                println!("[ERROR] Service: Database error getting assignment details: {:?}", e);
                return Err(e);
            }
        };

        println!("[DEBUG] Service: Extracted IDs - school: {}, enrollment: {}, assignment: {}, template: {}",
                 school_id, enrollment_id, student_form_assignment_id, form_template_id);
        println!("[DEBUG] Service: About to create form submission");

        // Create form submission with version control (returns tuple: (submission, is_insert))
        let (submission, is_insert) = match self.dao
            .create_form_submission_from_payload(
                actual_payload.clone(),
                school_id,
                enrollment_id,
                student_form_assignment_id,
                form_template_id
            )
            .await
        {
            Ok((sub, is_insert)) => {
                println!("[DEBUG] Service: Form submission {} successfully with ID: {} (is_insert: {})",
                         if is_insert { "created" } else { "updated" }, sub.id, is_insert);
                (sub, is_insert)
            }
            Err(e) => {
                println!("[ERROR] Service: Failed to create form submission: {:?}", e);
                return Err(e);
            }
        };

        // Notify admins every time a parent submits a form via Fillout — including
        // re-submissions, since each one needs review. The webhook only fires on
        // actual form submit (not on draft autosaves), so this won't spam admins.
        let submitted_event = if is_insert { "submitted" } else { "re-submitted" };
        let admin_body = match self
            .assignment_dao
            .get_assignment_notification_context(student_form_assignment_id)
            .await
        {
            Ok(ctx) => {
                let parent_full = format!(
                    "{} {}",
                    ctx.parent_first_name.trim(),
                    ctx.parent_last_name.trim()
                )
                .trim()
                .to_string();
                let classroom_suffix = match ctx.classroom_name.as_deref().filter(|s| !s.is_empty()) {
                    Some(c) => format!(" ({})", c),
                    None => String::new(),
                };
                format!(
                    "{} {} \"{}\" for {}{}. Please review and approve.",
                    if parent_full.is_empty() { "A parent".to_string() } else { parent_full },
                    submitted_event,
                    ctx.form_name,
                    ctx.child_full_name,
                    classroom_suffix,
                )
            }
            Err(e) => {
                eprintln!(
                    "[NotificationService] form_submitted context lookup failed for {}: {:?}",
                    student_form_assignment_id, e
                );
                format!(
                    "A parent {} a form. Please review and approve.",
                    submitted_event
                )
            }
        };

        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id,
                notification_type: notification_type::FORM_SUBMITTED.to_string(),
                title: "Pending Approval Form".to_string(),
                body: admin_body,
                related_entity_id: Some(student_form_assignment_id),
                related_entity_type: Some("form_assignment".to_string()),
                action_url: None,
            },
            None,
        ).await;

        // Extract fillout_submission_id from the submission for Fillout API call
        let fillout_submission_id = &submission.fillout_submission_id;
        println!("[DEBUG] Service: Extracted fillout_submission_id: {}", fillout_submission_id);

        // Extract form_id from the payload (optional for graceful degradation)
        let form_id_opt = actual_payload.get("form_id")
            .or_else(|| actual_payload.get("formId"))
            .and_then(|v| v.as_str());

        if let Some(form_id) = form_id_opt {
            println!("[DEBUG] Service: Extracted form_id: {}", form_id);

            // Fetch additional details from Fillout API if service is available
            if let Some(ref fillout_service) = self.fillout_service {
                println!("[DEBUG] Service: Fillout service available, fetching submission details");

                match fillout_service.fetch_submission_details(form_id, fillout_submission_id).await {
                Ok(fillout_details) => {
                    println!("[DEBUG] Service: Successfully fetched Fillout details - edit_link: {:?}, pdf_link: {:?}",
                             fillout_details.edit_link, fillout_details.pdf_link);

                    // Update form_submissions table with links
                    if let Err(e) = self.dao.update_submission_links(
                        submission.id,
                        fillout_details.edit_link.clone(),
                        fillout_details.pdf_link.clone(),
                    ).await {
                        println!("[WARN] Service: Failed to update submission links: {:?}", e);
                        // Don't fail the webhook - just log the warning
                    } else {
                        println!("[DEBUG] Service: Successfully updated submission links");
                    }

                    // Update student_form_assignments table with recent links
                    if let Err(e) = self.dao.update_assignment_links(
                        student_form_assignment_id,
                        fillout_details.edit_link.clone(),
                        fillout_details.pdf_link.clone(),
                    ).await {
                        println!("[WARN] Service: Failed to update assignment links: {:?}", e);
                        // Don't fail the webhook - just log the warning
                    } else {
                        println!("[DEBUG] Service: Successfully updated assignment links");
                    }

                    // Create enhanced response with the fetched links
                    let mut response = FormSubmissionResponse::from(submission);
                    response.edit_link = fillout_details.edit_link;
                    response.pdf_link = fillout_details.pdf_link;

                    println!("[DEBUG] Service: Created enhanced response with Fillout links");
                    Ok(response)
                }
                Err(e) => {
                    println!("[WARN] Service: Failed to fetch Fillout details: {:?}", e);
                    println!("[DEBUG] Service: Continuing with standard response (graceful degradation)");
                    // Graceful degradation - return the submission without links
                    Ok(submission.into())
                }
            }
            } else {
                println!("[DEBUG] Service: Fillout service not configured, skipping link fetching");
                Ok(submission.into())
            }
        } else {
            println!("[DEBUG] Service: form_id not provided in payload, skipping Fillout API integration");
            Ok(submission.into())
        }
    }

    pub async fn get_latest_form_submission(
        &self,
        school_id: Uuid,
        enrollment_id: Uuid,
        form_template_id: Uuid,
    ) -> Result<Option<FormSubmissionResponse>, AppError> {
        let submission = self.dao
            .get_latest_form_submission(school_id, enrollment_id, form_template_id)
            .await?;

        Ok(submission.map(|s| s.into()))
    }

    pub async fn get_all_form_submission_versions(
        &self,
        school_id: Uuid,
        enrollment_id: Uuid,
        form_template_id: Uuid,
    ) -> Result<Vec<FormSubmissionVersionResponse>, AppError> {
        let submissions = self.dao
            .get_all_form_submission_versions(school_id, enrollment_id, form_template_id)
            .await?;

        Ok(submissions.into_iter().map(|s| s.into()).collect())
    }

    pub async fn get_form_submission_by_id(
        &self,
        submission_id: Uuid,
    ) -> Result<FormSubmissionResponse, AppError> {
        let submission = self.dao
            .get_form_submission_by_id(submission_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Form submission not found".to_string()))?;

        Ok(submission.into())
    }

    pub async fn update_form_submission_status(
        &self,
        submission_id: Uuid,
        request: UpdateFormSubmissionStatusRequest,
    ) -> Result<FormSubmissionResponse, AppError> {
        println!("[DEBUG] Service: Starting form submission update");
        println!("[DEBUG] Service: Status: {:?}, Reason: {:?}, Form Data: {:?}, Metadata: {:?}",
                 request.status, request.reason, request.form_data.is_some(), request.metadata.is_some());

        let submission = self.dao
            .update_form_submission(
                submission_id,
                request.status,
                request.reason,
                request.form_data,
                request.metadata
            )
            .await?;

        println!("[DEBUG] Service: Form submission updated successfully");
        Ok(submission.into())
    }

    pub async fn get_form_resume_link(
        &self,
        assignment_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        println!("[DEBUG] Service: Getting resume link for assignment: {}", assignment_id);

        // Tier 1: check DB for already-stored recent_edit_link
        if let Ok(Some(link)) = self.dao.get_recent_edit_link_by_assignment(assignment_id).await {
            if !link.is_empty() {
                println!("[DEBUG] Service: Found resume link in DB (fast path)");
                return Ok(Some(link));
            }
        }

        // Tier 2: poll Fillout API for in-progress submission
        let fillout_service = match &self.fillout_service {
            Some(s) => s,
            None => {
                println!("[WARN] Service: Fillout service not configured, cannot poll for resume link");
                return Ok(None);
            }
        };

        let raw_form_id = match self.dao.get_fillout_form_id_by_assignment(assignment_id).await? {
            Some(id) if !id.is_empty() => id,
            _ => {
                return Err(AppError::NotFound("Form assignment not found or has no associated form".to_string()));
            }
        };

        // fillout_form_id may be stored as a full URL (e.g. "https://goddard.fillout.com/t/weBcGoVzL3us")
        // The Fillout API expects only the slug (e.g. "weBcGoVzL3us")
        let form_id = if raw_form_id.starts_with("http") {
            raw_form_id
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or(&raw_form_id)
                .to_string()
        } else {
            raw_form_id
        };

        println!("[DEBUG] Service: Polling Fillout API for in-progress submission (form: {})", form_id);
        let edit_link = fillout_service
            .get_inprogress_edit_link(&form_id, &assignment_id.to_string())
            .await?;

        if let Some(ref link) = edit_link {
            println!("[DEBUG] Service: Found edit link from Fillout, storing in DB");
            if let Err(e) = self.dao.update_assignment_links(assignment_id, Some(link.clone()), None).await {
                println!("[WARN] Service: Failed to cache resume link in DB: {:?}", e);
            }
        }

        Ok(edit_link)
    }

    pub async fn validate_webhook_secret(&self, api_key: &str) -> Result<(), AppError> {
        println!("[DEBUG] Service: Validating webhook secret");

        // Use the same API key validation as other endpoints
        let expected_api_key = match std::env::var("OWNER_API_KEY") {
            Ok(key) => {
                println!("[DEBUG] Service: OWNER_API_KEY found");
                key
            }
            Err(e) => {
                println!("[ERROR] Service: OWNER_API_KEY not configured: {:?}", e);
                return Err(AppError::Internal("OWNER_API_KEY not configured".to_string()));
            }
        };

        if api_key != expected_api_key {
            println!("[ERROR] Service: API key mismatch");
            return Err(AppError::Authentication("Invalid API key".to_string()));
        }

        println!("[DEBUG] Service: Webhook secret validation successful");
        Ok(())
    }
}