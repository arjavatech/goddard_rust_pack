use chrono::{DateTime, NaiveDate, Utc};
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

// =====================================================
// Parent lifecycle email notification payloads
// See docs/EMAIL_NOTIFICATIONS.md for the spec.
// =====================================================

#[derive(Debug, Clone)]
pub struct FormApprovedNotification {
    pub parent_email: String, // comma-separated allowed (primary + secondary)
    pub parent_first_name: String,
    pub child_name: String,
    pub form_name: String,
    pub reviewer_name: String,
    pub reviewed_on: DateTime<Utc>,
    pub notes: Option<String>,
    pub dashboard_url: String,
}

#[derive(Debug, Clone)]
pub struct FormRejectedNotification {
    pub parent_email: String,
    pub parent_first_name: String,
    pub child_name: String,
    pub form_name: String,
    pub reviewer_name: String,
    pub reviewed_on: DateTime<Utc>,
    pub notes: Option<String>,
    pub dashboard_url: String,
}

#[derive(Debug, Clone)]
pub struct ChildAddedNotification {
    pub parent_email: String,
    pub parent_first_name: String,
    pub child_name: String,
    pub child_dob: Option<NaiveDate>,
    pub classroom_name: String,
    pub school_name: String,
    pub added_on: DateTime<Utc>,
    pub form_count: usize,
    pub dashboard_url: String,
}

#[derive(Debug, Clone)]
pub struct ParentDeactivatedNotification {
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_full_name: String,
    pub school_name: String,
    pub deactivated_on: DateTime<Utc>,
    pub children_count: usize,
    pub enrollments_count: usize,
}

#[derive(Debug, Clone)]
pub struct ChildArchivedNotification {
    pub parent_email: String,
    pub parent_first_name: String,
    pub child_name: String,
    pub school_name: String,
    pub archived_on: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FormAssignedNotification {
    pub parent_email: String,
    pub parent_first_name: String,
    pub child_name: String,
    pub form_name: String,
    pub school_name: String,
    pub is_required: bool,
    pub due_date: Option<NaiveDate>,
    pub assigned_on: DateTime<Utc>,
    pub dashboard_url: String,
}
