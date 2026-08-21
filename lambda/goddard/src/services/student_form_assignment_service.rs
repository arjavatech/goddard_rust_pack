use crate::dao::StudentFormAssignmentDao;
use crate::models::student_form_assignment::{
    StudentFormAssignment, StudentFormAssignmentResponse, CreateStudentFormAssignmentRequest,
    UpdateStudentFormAssignmentRequest, DeleteStudentFormAssignmentResponse,
    BulkAssignFormRequest, BulkAssignFormResponse, FailedAssignment
};
use crate::models::student_form_assignment_review::{
    ReviewStudentFormAssignmentRequest, ReviewStudentFormAssignmentResponse
};
use crate::models::email::{FormApprovedNotification, FormAssignedNotification, FormRejectedNotification};
use crate::models::notification::{notification_type, CreateNotification};
use crate::services::email_service::{parent_dashboard_url, EmailService};
use crate::services::NotificationService;
use crate::error::AppError;
use crate::models::form_review_queue::{FormReviewQueueQuery, StudentFormReviewQueueItem};
use uuid::Uuid;
use std::collections::HashMap;
use std::io::{Write, Cursor};
use std::sync::Arc;
use chrono::Utc;
use zip::write::{SimpleFileOptions, ZipWriter};

pub struct StudentFormAssignmentService {
    dao: StudentFormAssignmentDao,
    email_service: Arc<EmailService>,
    notification_service: Arc<NotificationService>,
}

impl StudentFormAssignmentService {
    pub fn new(
        dao: StudentFormAssignmentDao,
        email_service: Arc<EmailService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            dao,
            email_service,
            notification_service,
        }
    }

    pub async fn create_student_form_assignment(
        &self,
        request: CreateStudentFormAssignmentRequest,
    ) -> Result<StudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment creation");
        println!("[DEBUG] StudentFormAssignmentService: Request data: {:?}", request);

        // Create the assignment
        let assignment = self.dao
            .create_student_form_assignment(&request)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment created successfully with ID: {}", assignment.id);

        self.fire_form_assigned_email(assignment.id).await;

        Ok(assignment.into())
    }

    pub async fn get_review_queue(&self, query: &FormReviewQueueQuery) -> Result<Vec<StudentFormReviewQueueItem>, AppError> {
        let mut items = self.dao.get_review_queue(query.school_id).await?;
        if let Some(classroom_id) = query.classroom_id {
            items.retain(|item| item.classroom_id == Some(classroom_id));
        }
        if let Some(form_template_id) = query.form_template_id {
            items.retain(|item| item.form_template_id == form_template_id);
        }
        if let Some(search) = query.search.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            let needle = search.to_lowercase();
            items.retain(|item| format!("{} {} {} {} {}", item.student_first_name, item.student_last_name, item.parent_first_name, item.parent_last_name, item.parent_email)
                .to_lowercase().contains(&needle));
        }
        let ascending = query.sort_direction.as_deref().map(|value| value.eq_ignore_ascii_case("asc")).unwrap_or(false);
        if query.sort_by.as_deref().map(|value| value.eq_ignore_ascii_case("name")).unwrap_or(false) {
            items.sort_by_key(|item| format!("{} {}", item.student_first_name.to_lowercase(), item.student_last_name.to_lowercase()));
        } else {
            items.sort_by_key(|item| item.submitted_at);
        }
        if !ascending { items.reverse(); }
        Ok(items)
    }

    /// Look up the recipient + form context for a freshly created assignment and
    /// dispatch the "new form assigned" email AND in-app notification on detached tasks.
    /// Errors are logged but never propagated. See docs/EMAIL_NOTIFICATIONS.md and
    /// docs/IN_APP_NOTIFICATIONS.md.
    async fn fire_form_assigned_email(&self, assignment_id: Uuid) {
        let ctx = match self.dao.get_assignment_notification_context(assignment_id).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[NotificationService] form_assigned context lookup failed for {}: {:?}",
                    assignment_id, e
                );
                return;
            }
        };
        let recipients = match ctx.secondary_parent_email.as_ref() {
            Some(sp) if !sp.trim().is_empty() => format!("{},{}", ctx.parent_email, sp),
            _ => ctx.parent_email.clone(),
        };

        // In-app: parent + secondary parent.
        let due_suffix = match ctx.due_date {
            Some(d) => format!(" Due: {}.", d.format("%B %d, %Y")),
            None => " Please complete it at your earliest convenience.".to_string(),
        };
        let body = format!(
            "A new form \"{}\" has been added to {}'s profile at {}.{}",
            ctx.form_name, ctx.child_full_name, ctx.school_name, due_suffix
        );
        self.notification_service.notify_user(
            ctx.parent_id,
            CreateNotification {
                school_id: ctx.school_id,
                notification_type: notification_type::FORM_ASSIGNED.to_string(),
                title: "Form Added to Child".to_string(),
                body: body.clone(),
                related_entity_id: Some(assignment_id),
                related_entity_type: Some("form_assignment".to_string()),
                action_url: Some("/dashboard".to_string()),
            },
        ).await;
        if let Some(sec_id) = ctx.secondary_parent_id {
            self.notification_service.notify_user(
                sec_id,
                CreateNotification {
                    school_id: ctx.school_id,
                    notification_type: notification_type::FORM_ASSIGNED.to_string(),
                    title: "Form Added to Child".to_string(),
                    body,
                    related_entity_id: Some(assignment_id),
                    related_entity_type: Some("form_assignment".to_string()),
                    action_url: Some("/dashboard".to_string()),
                },
            ).await;
        }

        let notification = FormAssignedNotification {
            parent_email: recipients,
            parent_first_name: ctx.parent_first_name,
            child_name: ctx.child_full_name,
            form_name: ctx.form_name,
            school_name: ctx.school_name,
            is_required: ctx.is_required,
            due_date: ctx.due_date,
            assigned_on: Utc::now(),
            dashboard_url: parent_dashboard_url(),
        };
        let email_svc = self.email_service.clone();
        tokio::spawn(async move {
            if let Err(e) = email_svc.send_form_assigned_email(notification).await {
                eprintln!(
                    "[EmailService] form_assigned notification failed (non-fatal): {:?}",
                    e
                );
            }
        });
    }

    pub async fn get_assignments_by_school(
        &self,
        school_id: Uuid,
    ) -> Result<Vec<StudentFormAssignmentResponse>, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Getting assignments for school: {}", school_id);

        let assignments = self.dao
            .get_assignments_by_school(school_id)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Found {} assignments", assignments.len());
        Ok(assignments.into_iter().map(|a| a.into()).collect())
    }

    pub async fn update_student_form_assignment(
        &self,
        request: UpdateStudentFormAssignmentRequest,
    ) -> Result<StudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment update");
        println!("[DEBUG] StudentFormAssignmentService: Update request: {:?}", request);

        let assignment = self.dao
            .update_student_form_assignment(&request)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment updated successfully");
        Ok(assignment.into())
    }

    pub async fn delete_student_form_assignment(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
    ) -> Result<DeleteStudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment deletion");
        println!("[DEBUG] StudentFormAssignmentService: Assignment ID: {}, School ID: {}", assignment_id, school_id);

        self.dao
            .delete_student_form_assignment(assignment_id, school_id)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment deleted successfully");
        Ok(DeleteStudentFormAssignmentResponse {
            message: "Student form assignment successfully deleted".to_string(),
            assignment_id,
            school_id,
        })
    }

    pub async fn review_student_form_assignment(
        &self,
        request: ReviewStudentFormAssignmentRequest,
    ) -> Result<ReviewStudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment review");
        println!("[DEBUG] StudentFormAssignmentService: Review request: {:?}", request);

        // Validate that the status is either Approved or Rejected
        match request.status {
            crate::models::student_form_assignment::StudentFormAssignmentStatus::Approved |
            crate::models::student_form_assignment::StudentFormAssignmentStatus::Rejected => {
                // Status is valid for review
            }
            _ => {
                println!("[ERROR] StudentFormAssignmentService: Invalid review status: {:?}", request.status);
                return Err(AppError::Validation("Review status must be 'approved' or 'rejected'".to_string()));
            }
        }

        let response = self.dao
            .review_student_form_assignment(&request)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment reviewed successfully");

        // Fire approval/rejection email (non-blocking). See docs/EMAIL_NOTIFICATIONS.md.
        match self
            .dao
            .get_review_notification_context(request.assignment_id, request.approved_by)
            .await
        {
            Ok(ctx) => {
                let recipients = match ctx.secondary_parent_email.as_ref() {
                    Some(sp) if !sp.trim().is_empty() => format!("{},{}", ctx.parent_email, sp),
                    _ => ctx.parent_email.clone(),
                };
                let reviewer_name = format!(
                    "{} {}",
                    ctx.reviewer_first_name.trim(),
                    ctx.reviewer_last_name.trim()
                )
                .trim()
                .to_string();
                let reviewer_name = if reviewer_name.is_empty() {
                    "Goddard School Admin".to_string()
                } else {
                    reviewer_name
                };
                let email_svc = self.email_service.clone();

                match request.status {
                    crate::models::student_form_assignment::StudentFormAssignmentStatus::Approved => {
                        let notification = FormApprovedNotification {
                            parent_email: recipients,
                            parent_first_name: ctx.parent_first_name,
                            child_name: ctx.child_full_name,
                            form_name: ctx.form_name,
                            reviewer_name,
                            reviewed_on: Utc::now(),
                            notes: request.notes.clone(),
                            dashboard_url: parent_dashboard_url(),
                        };
                        tokio::spawn(async move {
                            if let Err(e) = email_svc.send_form_approved_email(notification).await {
                                eprintln!("[EmailService] form_approved notification failed (non-fatal): {:?}", e);
                            }
                        });

                        // In-app, WebSocket, and FCM review notifications are intentionally
                        // disabled for now. Review emails above remain asynchronous.
                    }
                    crate::models::student_form_assignment::StudentFormAssignmentStatus::Rejected => {
                        let notification = FormRejectedNotification {
                            parent_email: recipients,
                            parent_first_name: ctx.parent_first_name,
                            child_name: ctx.child_full_name,
                            form_name: ctx.form_name,
                            reviewer_name,
                            reviewed_on: Utc::now(),
                            notes: request.notes.clone(),
                            dashboard_url: parent_dashboard_url(),
                        };
                        tokio::spawn(async move {
                            if let Err(e) = email_svc.send_form_rejected_email(notification).await {
                                eprintln!("[EmailService] form_rejected notification failed (non-fatal): {:?}", e);
                            }
                        });

                        // In-app, WebSocket, and FCM review notifications are intentionally
                        // disabled for now. Review emails above remain asynchronous.
                    }
                    _ => {}
                }
            }
            Err(e) => {
                println!(
                    "[StudentFormAssignmentService] Skipping review email — context lookup failed: {:?}",
                    e
                );
            }
        }

        Ok(response)
    }

    pub async fn validate_api_key(&self, api_key: &str) -> Result<(), AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Validating API key");

        // Use the same API key validation as other endpoints
        let expected_api_key = match std::env::var("OWNER_API_KEY") {
            Ok(key) => {
                println!("[DEBUG] StudentFormAssignmentService: OWNER_API_KEY found");
                key
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: OWNER_API_KEY not configured: {:?}", e);
                return Err(AppError::Internal("OWNER_API_KEY not configured".to_string()));
            }
        };

        if api_key != expected_api_key {
            println!("[ERROR] StudentFormAssignmentService: API key mismatch");
            return Err(AppError::Authentication("Invalid API key".to_string()));
        }

        println!("[DEBUG] StudentFormAssignmentService: API key validation successful");
        Ok(())
    }

    pub async fn bulk_assign_forms(
        &self,
        request: BulkAssignFormRequest,
    ) -> Result<BulkAssignFormResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting bulk form assignment");
        println!("[DEBUG] StudentFormAssignmentService: School ID: {}, Number of assignments: {}",
                 request.school_id, request.assignments.len());

        // Validate request has at least one assignment
        if request.assignments.is_empty() {
            println!("[ERROR] StudentFormAssignmentService: No assignments provided");
            return Err(AppError::Validation("At least one assignment is required".to_string()));
        }

        // Extract unique form template IDs for validation
        let form_template_ids: Vec<Uuid> = request.assignments
            .iter()
            .map(|a| a.form_template_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        println!("[DEBUG] StudentFormAssignmentService: Validating {} unique form templates", form_template_ids.len());

        // Step 1: Validate all form templates are active
        match self.dao.validate_form_templates_active(&form_template_ids).await {
            Ok(_) => println!("[DEBUG] StudentFormAssignmentService: All form templates validated as active"),
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: Form template validation failed: {:?}", e);
                return Err(e);
            }
        }

        // Step 2: Check for duplicate assignments
        match self.dao.check_duplicate_assignments(request.school_id, &request.assignments).await {
            Ok(_) => println!("[DEBUG] StudentFormAssignmentService: No duplicate assignments found"),
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: Duplicate assignment check failed: {:?}", e);
                return Err(e);
            }
        }

        // Step 3: Create assignments in bulk (within transaction)
        match self.dao.bulk_create_assignments(request.school_id, request.assignments).await {
            Ok(created_assignments) => {
                println!("[DEBUG] StudentFormAssignmentService: Successfully created {} assignments", created_assignments.len());

                for assignment in &created_assignments {
                    self.fire_form_assigned_email(assignment.id).await;
                }

                let successful: Vec<StudentFormAssignmentResponse> = created_assignments
                    .into_iter()
                    .map(|a| a.into())
                    .collect();

                Ok(BulkAssignFormResponse {
                    successful,
                    failed: Vec::new(), // No failures in current implementation
                })
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: Bulk creation failed: {:?}", e);
                Err(e)
            }
        }
    }

    pub async fn get_enrollment_parent_id(&self, enrollment_id: Uuid) -> Result<(Uuid, String, String), AppError> {
        self.dao.get_enrollment_parent_id(enrollment_id).await
    }

    pub async fn download_enrollment_forms_zip(
        &self,
        enrollment_id: Uuid,
        child_first_name: &str,
        child_last_name: &str,
    ) -> Result<(Vec<u8>, String), AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Downloading forms ZIP for enrollment: {}", enrollment_id);

        let forms = self.dao.get_completed_assignments_for_zip(enrollment_id).await?;

        if forms.is_empty() {
            return Err(AppError::NotFound("No completed forms with PDF links found for this enrollment".to_string()));
        }

        println!("[DEBUG] StudentFormAssignmentService: Found {} forms to download", forms.len());

        let client = reqwest::Client::new();
        let mut buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut name_counts: HashMap<String, u32> = HashMap::new();
        let mut success_count = 0u32;

        for form in &forms {
            let sanitized = Self::sanitize_filename(&form.form_name);
            let count = name_counts.entry(sanitized.clone()).or_insert(0);
            *count += 1;
            let file_name = if *count == 1 {
                format!("{}.pdf", sanitized)
            } else {
                format!("{}_{}.pdf", sanitized, count)
            };

            match client.get(&form.recent_pdf_link).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes().await {
                            Ok(bytes) => {
                                if let Err(e) = zip.start_file(&file_name, options) {
                                    println!("[WARN] Failed to start ZIP entry for {}: {}", file_name, e);
                                    continue;
                                }
                                if let Err(e) = zip.write_all(&bytes) {
                                    println!("[WARN] Failed to write ZIP entry for {}: {}", file_name, e);
                                    continue;
                                }
                                success_count += 1;
                                println!("[DEBUG] Added to ZIP: {}", file_name);
                            }
                            Err(e) => {
                                println!("[WARN] Failed to read PDF bytes for {}: {}", form.form_name, e);
                            }
                        }
                    } else {
                        println!("[WARN] PDF download returned status {} for {}", resp.status(), form.form_name);
                    }
                }
                Err(e) => {
                    println!("[WARN] Failed to download PDF for {}: {}", form.form_name, e);
                }
            }
        }

        if success_count == 0 {
            return Err(AppError::ExternalService("All PDF downloads failed".to_string()));
        }

        zip.finish().map_err(|e| AppError::Internal(format!("Failed to finalize ZIP: {}", e)))?;

        let zip_bytes = buffer.into_inner();
        let sanitized_first = Self::sanitize_filename(child_first_name);
        let sanitized_last = Self::sanitize_filename(child_last_name);
        let filename = format!("{}_{}_{}.zip", sanitized_first, sanitized_last, "completed_forms");

        println!("[DEBUG] StudentFormAssignmentService: ZIP created with {} of {} forms", success_count, forms.len());
        Ok((zip_bytes, filename))
    }

    fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect::<String>()
            .trim()
            .replace(' ', "_")
    }

    /// Assign a form template to all active students in a school
    pub async fn assign_form_to_school_students(
        &self,
        request: crate::models::student_form_assignment::AssignFormToSchoolStudentsRequest,
    ) -> Result<crate::models::student_form_assignment::AssignFormToSchoolStudentsResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Assigning form {} to all active students in school {}",
            request.form_template_id, request.school_id);

        // Validate that the form template is active
        let form_template_ids = vec![request.form_template_id];
        self.dao.validate_form_templates_active(&form_template_ids).await?;
        println!("[DEBUG] StudentFormAssignmentService: Form template validated");

        // Call DAO to assign forms to all active students
        let is_required = request.is_required.unwrap_or(false);
        let (created_assignments, total_active_students, students_already_assigned) = self.dao
            .assign_form_to_school_students(
                request.school_id,
                request.form_template_id,
                is_required,
            )
            .await?;

        for assignment in &created_assignments {
            self.fire_form_assigned_email(assignment.id).await;
        }

        // Convert to response DTOs
        let successful: Vec<crate::models::student_form_assignment::StudentFormAssignmentResponse> =
            created_assignments.into_iter()
                .map(|assignment| assignment.into())
                .collect();

        let newly_assigned = successful.len() as i64;

        println!("[DEBUG] StudentFormAssignmentService: Assignment complete. Total: {}, Already assigned: {}, Newly assigned: {}",
            total_active_students, students_already_assigned, newly_assigned);

        Ok(crate::models::student_form_assignment::AssignFormToSchoolStudentsResponse {
            school_id: request.school_id,
            form_template_id: request.form_template_id,
            total_active_students,
            students_already_assigned,
            newly_assigned,
            failed_assignments: 0,
            successful,
            failed: Vec::new(),
        })
    }

    /// Assign a form template to all active students in a specific class
    pub async fn assign_form_to_class_students(
        &self,
        request: crate::models::student_form_assignment::AssignFormToClassStudentsRequest,
    ) -> Result<crate::models::student_form_assignment::AssignFormToClassStudentsResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Assigning form {} to all active students in class {} of school {}",
            request.form_template_id, request.class_id, request.school_id);

        // Validate that the form template is active
        let form_template_ids = vec![request.form_template_id];
        self.dao.validate_form_templates_active(&form_template_ids).await?;
        println!("[DEBUG] StudentFormAssignmentService: Form template validated");

        // Call DAO to assign forms to all active students in the class
        let is_required = false;
        let (created_assignments, total_active_students, students_already_assigned) = self.dao
            .assign_form_to_class_students(
                request.school_id,
                request.class_id,
                request.form_template_id,
                is_required,
            )
            .await?;

        for assignment in &created_assignments {
            self.fire_form_assigned_email(assignment.id).await;
        }

        // Convert to response DTOs
        let successful: Vec<crate::models::student_form_assignment::StudentFormAssignmentResponse> =
            created_assignments.into_iter()
                .map(|assignment| assignment.into())
                .collect();

        let newly_assigned = successful.len() as i64;

        println!("[DEBUG] StudentFormAssignmentService: Class assignment complete. Total: {}, Already assigned: {}, Newly assigned: {}",
            total_active_students, students_already_assigned, newly_assigned);

        Ok(crate::models::student_form_assignment::AssignFormToClassStudentsResponse {
            school_id: request.school_id,
            class_id: request.class_id,
            form_template_id: request.form_template_id,
            total_active_students,
            students_already_assigned,
            newly_assigned,
            failed_assignments: 0,
            successful,
            failed: Vec::new(),
        })
    }
}
