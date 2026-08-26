use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub school_id: Uuid,
    pub audience: String,
    pub document_name: String,
    pub instructions: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub target: String,
    pub child_ids: Option<Vec<Uuid>>,
    pub employee_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentRequestQuery {
    pub school_id: Uuid,
    pub audience: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentAssignmentQuery {
    pub school_id: Uuid,
    pub audience: Option<String>,
    pub request_id: Option<Uuid>,
    pub assignment_id: Option<Uuid>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentReminderRequest {
    pub school_id: Uuid,
    pub assignment_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DocumentReminder {
    pub assignment_id: Uuid,
    pub audience: String,
    pub recipient_email: String,
    pub recipient_name: String,
    pub subject_name: String,
    pub classroom_name: Option<String>,
    pub document_name: String,
    pub due_date: Option<NaiveDate>,
    pub rejection_reason: Option<String>,
    pub is_overdue: bool,
}

#[derive(Debug, Serialize)]
pub struct DocumentRecipient {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub classroom_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadIntentRequest {
    pub file_name: String,
    pub content_type: String,
    pub file_size_bytes: i64,
}

#[derive(Debug, Deserialize)]
pub struct CompleteUploadRequest {
    pub storage_key: String,
    pub file_name: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewDocumentAssignmentRequest {
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DocumentRequestSummary {
    pub id: Uuid,
    pub school_id: Uuid,
    pub audience: String,
    pub document_name: String,
    pub instructions: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: String,
    pub published_at: Option<NaiveDateTime>,
    pub submitted: i64,
    pub pending: i64,
    pub approved: i64,
    pub rejected: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct DocumentAssignmentItem {
    pub id: Uuid,
    pub request_id: Uuid,
    pub school_id: Uuid,
    pub audience: String,
    pub document_name: String,
    pub instructions: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub request_status: String,
    pub status: String,
    pub derived_status: String,
    pub subject_name: String,
    pub parent_name: Option<String>,
    pub parent_email: Option<String>,
    pub classroom_name: Option<String>,
    pub employee_email: Option<String>,
    pub submitted_at: Option<NaiveDateTime>,
    pub reviewed_at: Option<NaiveDateTime>,
    pub rejection_reason: Option<String>,
    pub latest_submission_id: Option<Uuid>,
    pub latest_file_name: Option<String>,
    pub latest_content_type: Option<String>,
    pub latest_file_size_bytes: Option<i64>,
    pub version_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PagedDocumentAssignments {
    pub items: Vec<DocumentAssignmentItem>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct UploadIntentResponse {
    pub storage_key: String,
    pub upload_url: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct FileAccessResponse { pub url: String, pub expires_in_seconds: u64 }

#[derive(Debug, Serialize)]
pub struct DocumentAuditEvent {
    pub id: Uuid,
    pub event_type: String,
    pub actor_name: Option<String>,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
}
