use crate::dao::FormSubmissionDao;
use crate::models::form_submission::{
    CreateFormSubmissionWebhookRequest, FormSubmission, FormSubmissionResponse,
    FormSubmissionStatus, FormSubmissionVersionResponse, UpdateFormSubmissionStatusRequest,
};
use crate::error::AppError;
use uuid::Uuid;

pub struct FormSubmissionService {
    dao: FormSubmissionDao,
}

impl FormSubmissionService {
    pub fn new(dao: FormSubmissionDao) -> Self {
        Self { dao }
    }

    pub async fn create_form_submission_from_webhook(
        &self,
        request: CreateFormSubmissionWebhookRequest,
    ) -> Result<FormSubmissionResponse, AppError> {
        println!("[DEBUG] Service: Starting webhook processing");
        println!("[DEBUG] Service: Request student_form_assignment_id: {}", request.student_form_assignment_id);

        // Get form template ID from student form assignment
        let form_template_id = match self.dao
            .get_form_template_id_from_assignment(request.student_form_assignment_id)
            .await
        {
            Ok(Some(id)) => {
                println!("[DEBUG] Service: Found form_template_id: {}", id);
                id
            }
            Ok(None) => {
                println!("[ERROR] Service: Student form assignment not found: {}", request.student_form_assignment_id);
                return Err(AppError::NotFound("Student form assignment not found".to_string()));
            }
            Err(e) => {
                println!("[ERROR] Service: Database error getting form template ID: {:?}", e);
                return Err(e);
            }
        };

        println!("[DEBUG] Service: About to create form submission");

        // Create form submission with version control
        let submission = match self.dao
            .create_form_submission(&request, form_template_id)
            .await
        {
            Ok(sub) => {
                println!("[DEBUG] Service: Form submission created successfully with ID: {}", sub.id);
                sub
            }
            Err(e) => {
                println!("[ERROR] Service: Failed to create form submission: {:?}", e);
                return Err(e);
            }
        };

        println!("[DEBUG] Service: Converting to response format");
        Ok(submission.into())
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