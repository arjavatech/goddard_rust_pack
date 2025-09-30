use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::student_form_assignment::StudentFormAssignmentStatus;

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewStudentFormAssignmentRequest {
    pub assignment_id: Uuid,
    pub status: StudentFormAssignmentStatus, // Should be Approved or Rejected
    pub notes: Option<String>,
    pub approved_by: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewStudentFormAssignmentResponse {
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