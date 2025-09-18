use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormSubmission {
    pub id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub student_form_assignment_id: Uuid,
    pub form_template_id: Uuid,
    pub fillout_submission_id: String,
    pub form_data: JsonValue,
    pub metadata: JsonValue,
    pub status: FormSubmissionStatus,
    pub revision_number: i32,
    pub revision_reason: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FormSubmissionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    RequiresReview,
    Approved,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateFormSubmissionWebhookRequest {
    pub form_id: String,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub student_form_assignment_id: Uuid,
    pub form_data: JsonValue,
    pub metadata: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FormSubmissionResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub student_form_assignment_id: Uuid,
    pub form_template_id: Uuid,
    pub fillout_submission_id: String,
    pub form_data: JsonValue,
    pub metadata: JsonValue,
    pub status: FormSubmissionStatus,
    pub revision_number: i32,
    pub revision_reason: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FormSubmissionVersionResponse {
    pub id: Uuid,
    pub revision_number: i32,
    pub revision_reason: Option<String>,
    pub form_data: JsonValue,
    pub metadata: JsonValue,
    pub status: FormSubmissionStatus,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateFormSubmissionStatusRequest {
    pub status: Option<FormSubmissionStatus>,
    pub reason: Option<String>,
    pub form_data: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetFormSubmissionQuery {
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub form_template_id: Uuid,
}

impl From<FormSubmission> for FormSubmissionResponse {
    fn from(submission: FormSubmission) -> Self {
        Self {
            id: submission.id,
            school_id: submission.school_id,
            enrollment_id: submission.enrollment_id,
            student_form_assignment_id: submission.student_form_assignment_id,
            form_template_id: submission.form_template_id,
            fillout_submission_id: submission.fillout_submission_id,
            form_data: submission.form_data,
            metadata: submission.metadata,
            status: submission.status,
            revision_number: submission.revision_number,
            revision_reason: submission.revision_reason,
            submitted_at: submission.submitted_at,
            processed_at: submission.processed_at,
        }
    }
}

impl From<FormSubmission> for FormSubmissionVersionResponse {
    fn from(submission: FormSubmission) -> Self {
        Self {
            id: submission.id,
            revision_number: submission.revision_number,
            revision_reason: submission.revision_reason,
            form_data: submission.form_data,
            metadata: submission.metadata,
            status: submission.status,
            submitted_at: submission.submitted_at,
            processed_at: submission.processed_at,
        }
    }
}