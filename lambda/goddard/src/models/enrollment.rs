use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ParentInviteRequest {
    pub school_id: Uuid,
    pub child_first_name: String,
    pub child_last_name: String,
    pub child_birth_date: NaiveDate,
    pub gender: String,
    pub class_id: Uuid,
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
}

#[derive(Debug, Serialize)]
pub struct ParentInviteResponse {
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub enrollment_id: Uuid,
    pub assigned_forms_count: usize,
    pub invite_id: Uuid,
    pub signup_email_sent: bool,
    pub message: String,
    pub details: ParentInviteDetails,
}

#[derive(Debug, Serialize)]
pub struct ParentInviteDetails {
    pub parent: ParentDetails,
    pub child: ChildDetails,
    pub enrollment: EnrollmentDetails,
    pub assigned_forms: Vec<AssignedFormDetails>,
}

#[derive(Debug, Serialize)]
pub struct ParentDetails {
    pub id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
    pub is_verified: bool,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct ChildDetails {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: NaiveDate,
    pub gender: String,
    pub status: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentDetails {
    pub id: Uuid,
    pub child_id: Uuid,
    pub school_id: Uuid,
    pub classroom_id: Uuid,
    pub status: String,
    pub application_status: Option<Value>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct AssignedFormDetails {
    pub id: Uuid,
    pub form_template_id: Uuid,
    pub form_name: String,
    pub assignment_source: String,
    pub status: String,
    pub is_required: bool,
}

// Internal structures for database operations
#[derive(Debug, Clone)]
pub struct AuthUserResult {
    pub auth_user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct CreatedUser {
    pub id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
    pub is_verified: bool,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct CreatedChild {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: NaiveDate,
    pub gender: String,
    pub status: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct CreatedEnrollment {
    pub id: Uuid,
    pub child_id: Uuid,
    pub school_id: Uuid,
    pub classroom_id: Uuid,
    pub status: String,
    pub application_status: Option<Value>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct FormTemplate {
    pub id: Uuid,
    pub form_name: String,
    pub is_required: bool,
}

#[derive(Debug, Clone)]
pub struct ClassFormOverride {
    pub id: Uuid,
    pub form_template_id: Uuid,
    pub form_name: String,
    pub action: Option<String>, // "add" or "remove"
    pub is_required: bool,
}

#[derive(Debug, Clone)]
pub struct CreatedFormAssignment {
    pub id: Uuid,
    pub form_template_id: Uuid,
    pub form_name: String,
    pub assignment_source: String,
    pub status: String,
    pub is_required: bool,
}