use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::student_form_assignment::StudentFormAssignmentStatus;

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeStudentFormAssignmentRequest {
    pub assignment_id: Uuid,
    pub notes: String, // Required - reason for revocation
    pub revoked_by: Uuid,
    #[serde(default)]
    pub target_status: Option<String>, // "in_progress" | "rejected"; defaults to "in_progress"
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeStudentFormAssignmentResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub form_template_id: Uuid,
    pub assignment_source: String,
    pub status: StudentFormAssignmentStatus,
    pub is_required: bool,
    pub assigned_at: DateTime<Utc>,
    pub notes: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_on: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
