use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct ParentFormReminder {
    pub parent_email: String,
    pub parent_name: String,
    pub student_name: String,
    pub class_name: String,
    pub form_name: String,
    pub due_date: String, // DD-MM-YYYY format from API
}

#[derive(Debug, Deserialize)]
pub struct BulkEmailRequest {
    pub school_id: Uuid,
    pub reminders: Vec<ParentFormReminder>,
}

#[derive(Debug, Serialize)]
pub struct BulkEmailResponse {
    pub total_sent: usize,
    pub total_failed: usize,
    pub failed_emails: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
struct ResendEmailRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    html: String,
}
