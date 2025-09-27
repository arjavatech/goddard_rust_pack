use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveDateTime, DateTime, Utc};
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

#[derive(Debug, Deserialize)]
pub struct ResendConfirmationRequest {
    pub parent_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ResendConfirmationResponse {
    pub parent_id: Uuid,
    pub email_sent: bool,
    pub message: String,
    pub parent_details: ResendConfirmationParentDetails,
}

#[derive(Debug, Serialize)]
pub struct ResendConfirmationParentDetails {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AddChildRequest {
    pub school_id: Uuid,
    pub child_first_name: String,
    pub child_last_name: String,
    pub child_birth_date: NaiveDate,
    pub gender: String,
    pub class_id: Uuid,
    pub parent_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AddChildResponse {
    pub child_id: Uuid,
    pub enrollment_id: Uuid,
    pub assigned_forms_count: usize,
    pub message: String,
    pub details: AddChildDetails,
}

#[derive(Debug, Serialize)]
pub struct AddChildDetails {
    pub parent: AddChildParentDetails,
    pub child: ChildDetails,
    pub enrollment: EnrollmentDetails,
    pub assigned_forms: Vec<AssignedFormDetails>,
}

#[derive(Debug, Serialize)]
pub struct AddChildParentDetails {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub is_verified: bool,
}

#[derive(Debug, Deserialize)]
pub struct GetParentDetailsBySchoolRequest {
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct GetParentDetailsBySchoolResponse {
    pub parents: Vec<ParentWithChildren>,
}

#[derive(Debug, Serialize)]
pub struct ParentWithChildren {
    pub parent_id: Uuid,
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub children: Vec<ChildWithForms>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChildWithForms {
    pub child_id: Uuid,
    pub child_full_name: String,
    pub child_dob: NaiveDate,
    pub enrollment_id: Uuid,
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub forms: Vec<FormStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormStatus {
    pub form_id: String,
    pub fillout_form_id: Option<String>,
    pub form_name: String,
    pub status: String,
    pub is_required: bool,
}

#[derive(Debug, Serialize)]
pub struct ParentWithAuthDetails {
    pub id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub id_signed: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthUserDetails {
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub id_signed: bool,
}

// 8.6 Get Enrollment Children with Form Assignments
#[derive(Debug, Deserialize)]
pub struct GetEnrollmentChildrenRequest {
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetEnrollmentChildrenResponse {
    pub children: Vec<EnrollmentChildWithForms>,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentChildWithForms {
    pub child_id: Uuid,
    pub child_first_name: String,
    pub child_last_name: String,
    pub class_name: String,
    pub primary_email: String,
    pub form_status: String,
    pub forms: serde_json::Value, // JSON object with form_template_id as key and form_name as value
    pub additional_parent_email: Option<String>,
}

// 8.4 Get All Enrollment Form Details by School
#[derive(Debug, Deserialize)]
pub struct GetSchoolFormsRequest {
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetSchoolFormsResponse {
    pub enrollments: Vec<SchoolFormDetails>,
}

#[derive(Debug, Serialize)]
pub struct SchoolFormDetails {
    pub child_id: Uuid,
    pub child_first_name: String,
    pub child_last_name: String,
    pub class_name: String,
    pub primary_email: String,
    pub form_status: String,
    pub forms: serde_json::Value, // JSON object with form_template_id as key and form_name as value
    pub additional_parent_email: Option<String>,
}

// 8.5 Get Class-wise Child Count Details
#[derive(Debug, Deserialize)]
pub struct GetClassWiseCountRequest {
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetClassWiseCountResponse {
    pub classes: Vec<ClassWiseCount>,
}

#[derive(Debug, Serialize)]
pub struct ClassWiseCount {
    pub class_id: Uuid,
    pub class_name: String,
    pub count: i64,
    pub forms: serde_json::Value,
    pub default_forms: String,
}