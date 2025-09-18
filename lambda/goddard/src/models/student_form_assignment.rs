use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StudentFormAssignmentStatus {
    Incomplete,
    InProgress,
    Completed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StudentFormAssignment {
    pub id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub form_template_id: Uuid,
    pub assignment_source: String,
    pub status: StudentFormAssignmentStatus,
    pub is_required: bool,
    pub assigned_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateStudentFormAssignmentRequest {
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub form_template_id: Uuid,
    pub assignment_source: String,
    pub status: Option<StudentFormAssignmentStatus>,
    pub is_required: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateStudentFormAssignmentRequest {
    pub id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Option<Uuid>,
    pub child_id: Option<Uuid>,
    pub form_template_id: Option<Uuid>,
    pub assignment_source: Option<String>,
    pub status: Option<StudentFormAssignmentStatus>,
    pub is_required: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentFormAssignmentResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub form_template_id: Uuid,
    pub assignment_source: String,
    pub status: StudentFormAssignmentStatus,
    pub is_required: bool,
    pub assigned_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetStudentFormAssignmentsQuery {
    pub school_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteStudentFormAssignmentQuery {
    pub assignment_id: Uuid,
    pub school_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteStudentFormAssignmentResponse {
    pub message: String,
    pub assignment_id: Uuid,
    pub school_id: Uuid,
}

impl From<StudentFormAssignment> for StudentFormAssignmentResponse {
    fn from(assignment: StudentFormAssignment) -> Self {
        Self {
            id: assignment.id,
            school_id: assignment.school_id,
            enrollment_id: assignment.enrollment_id,
            child_id: assignment.child_id,
            form_template_id: assignment.form_template_id,
            assignment_source: assignment.assignment_source,
            status: assignment.status,
            is_required: assignment.is_required,
            assigned_at: assignment.assigned_at,
            updated_at: assignment.updated_at,
        }
    }
}