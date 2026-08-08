use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveDateTime, DateTime, Utc};
use uuid::Uuid;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ParentInviteRequest {
    pub school_id: Uuid,
    pub child_first_name: String,
    pub child_last_name: String,
    pub child_birth_date: Option<NaiveDate>,
    pub gender: String,
    pub class_id: Uuid,
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub parent_address: Option<String>,
    pub parent_phone_number: Option<String>,
    pub parent_relation_type: Option<String>,
    // Optional secondary parent fields
    pub secondary_parent_email: Option<String>,
    pub secondary_parent_first_name: Option<String>,
    pub secondary_parent_last_name: Option<String>,
    pub secondary_parent_address: Option<String>,
    pub secondary_parent_phone_number: Option<String>,
    pub secondary_parent_relation_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParentInviteResponse {
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub enrollment_id: Uuid,
    pub assigned_forms_count: usize,
    pub invite_id: Uuid,
    pub signup_email_sent: bool,
    // Secondary parent fields (optional)
    pub secondary_parent_id: Option<Uuid>,
    pub secondary_signup_email_sent: Option<bool>,
    pub message: String,
    pub details: ParentInviteDetails,
}

#[derive(Debug, Serialize)]
pub struct ParentInviteDetails {
    pub parent: ParentDetails,
    pub secondary_parent: Option<ParentDetails>,
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
    pub address: Option<String>,
    pub phone_number: Option<String>,
    pub relation_type: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct ChildDetails {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub secondary_parent_id: Option<Uuid>,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: Option<NaiveDate>,
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
    pub email_sent: bool,
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
    pub address: Option<String>,
    pub phone_number: Option<String>,
    pub relation_type: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct CreatedChild {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub secondary_parent_id: Option<Uuid>,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: Option<NaiveDate>,
    pub gender: Option<String>,
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
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
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
    pub child_birth_date: Option<NaiveDate>,
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
pub struct GetParentDetailsBySchoolResponse {
    pub active_parents: Vec<ParentWithChildren>,
    pub inactive_parents: Vec<ParentWithChildren>,
}

#[derive(Debug, Serialize)]
pub struct ParentWithChildren {
    pub parent_id: Uuid,
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub parent_address: Option<String>,
    pub parent_phone_number: Option<String>,
    pub parent_relation_type: Option<String>,
    pub signed_status: String,
    pub children: Vec<ChildWithForms>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChildWithForms {
    pub child_id: Uuid,
    pub child_full_name: String,
    pub child_dob: Option<NaiveDate>,
    pub child_status: String,
    pub gender: Option<String>,
    pub enrollment_id: Uuid,
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub forms: Vec<FormStatus>,
    // Primary parent info
    pub primary_parent_id: Uuid,
    pub primary_parent_first_name: String,
    pub primary_parent_last_name: String,
    pub primary_parent_email: String,
    // Secondary parent info (optional)
    pub secondary_parent_id: Option<Uuid>,
    pub secondary_parent_first_name: Option<String>,
    pub secondary_parent_last_name: Option<String>,
    pub secondary_parent_email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormStatus {
    pub form_id: String,
    pub student_form_assignment_id: Option<Uuid>,
    pub fillout_form_id: Option<String>,
    pub form_name: String,
    pub due_date: Option<String>,
    pub status: String,
    pub is_required: bool,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_on: Option<NaiveDateTime>,
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
    pub enrollment_id: Uuid,
    pub parent_id: Uuid,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub parent_address: Option<String>,
    pub parent_phone_number: Option<String>,
    pub parent_relation_type: Option<String>,
    pub child_id: Uuid,
    pub child_first_name: String,
    pub child_last_name: String,
    pub child_status: String,
    pub class_name: String,
    pub primary_email: String,
    pub form_status: String,
    /// Forms assigned to this enrollment
    /// Format: { "form_name": { "status": "incomplete|in_progress|completed|approved|rejected", "assigned_at": "DD-MM-YYYY" } }
    /// Example: { "Admission Form": { "status": "incomplete", "assigned_at": "26-09-2025" } }
    pub forms: serde_json::Value,
    pub additional_parent_email: Option<String>,
    // Secondary parent details
    pub secondary_parent_id: Option<Uuid>,
    pub secondary_parent_first_name: Option<String>,
    pub secondary_parent_last_name: Option<String>,
    pub secondary_parent_email: Option<String>,
    pub secondary_parent_address: Option<String>,
    pub secondary_parent_phone_number: Option<String>,
    pub secondary_parent_relation_type: Option<String>,
}

// 8.4.1 Get Class-Based Enrollment Form Details
#[derive(Debug, Deserialize)]
pub struct GetClassBasedEnrollmentsRequest {
    pub school_id: Uuid,
    pub class_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetClassBasedEnrollmentsResponse {
    pub enrollments: Vec<SchoolFormDetails>,
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
}

// Deactivate Parent Response
#[derive(Debug, Serialize)]
pub struct DeactivateParentResponse {
    pub parent_id: Uuid,
    pub deactivated_children_count: usize,
    pub deactivated_enrollments_count: usize,
    pub message: String,
}

// Activate Parent Response
#[derive(Debug, Serialize)]
pub struct ActivateParentResponse {
    pub parent_id: Uuid,
    pub activated_children_count: usize,
    pub activated_enrollments_count: usize,
    pub message: String,
}

// Update Child Status Request/Response
#[derive(Debug, Deserialize)]
pub struct UpdateChildStatusRequest {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateChildStatusResponse {
    pub child_id: Uuid,
    pub status: String,
    pub updated_at: NaiveDateTime,
    pub message: String,
}

// ==========================================
// CLASS TRANSITIONS API MODELS
// ==========================================

// ClassTransition model (database record)
#[derive(Debug, Serialize, Clone)]
pub struct ClassTransition {
    pub id: Uuid,
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub school_id: Uuid,
    pub from_classroom_id: Uuid,
    pub to_classroom_id: Uuid,
    pub changed_by: Option<Uuid>,
    pub reason: Option<String>,
    pub transitioned_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

// API 1: Promote Enrollment - Request/Response
#[derive(Debug, Deserialize)]
pub struct PromoteEnrollmentRequest {
    pub to_classroom_id: Uuid,
    pub reason: Option<String>,
    pub effective_date: Option<DateTime<Utc>>,
}

// Bulk Promotion Models
#[derive(Debug, Deserialize)]
pub struct PromotionRequest {
    pub enrollment_id: Uuid,
    pub to_classroom_id: Uuid,
    pub reason: Option<String>,
    pub effective_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct BulkPromoteEnrollmentsRequest {
    pub school_id: Uuid,
    pub promotions: Vec<PromotionRequest>,
}

#[derive(Debug, Serialize)]
pub struct FailedPromotion {
    pub enrollment_id: Uuid,
    pub child_name: Option<String>,
    pub to_classroom_id: Uuid,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct BulkPromoteEnrollmentsResponse {
    pub successful: Vec<PromoteEnrollmentResponse>,
    pub failed: Vec<FailedPromotion>,
    pub summary: PromotionSummary,
}

#[derive(Debug, Serialize)]
pub struct PromotionSummary {
    pub total_requested: usize,
    pub successful_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Serialize)]
pub struct PromoteEnrollmentResponse {
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub child_name: String,
    pub from_classroom: ClassroomInfo,
    pub to_classroom: ClassroomInfo,
    pub transition: TransitionInfo,
    pub message: String,
}

// API 2: Edit Class Transition - Request/Response
#[derive(Debug, Deserialize)]
pub struct EditClassTransitionRequest {
    pub to_classroom_id: Option<Uuid>,
    pub reason: Option<String>,
    pub transitioned_at: Option<DateTime<Utc>>,
    pub sync_enrollment: Option<bool>,  // Default: false
}

#[derive(Debug, Serialize)]
pub struct EditClassTransitionResponse {
    pub transition_id: Uuid,
    pub enrollment_id: Uuid,
    pub child_name: String,
    pub from_classroom: ClassroomInfo,
    pub to_classroom: ClassroomInfo,
    pub transitioned_at: NaiveDateTime,
    pub reason: Option<String>,
    pub enrollment_synced: bool,
    pub message: String,
}

// Shared helper structs
#[derive(Debug, Serialize)]
pub struct ClassroomInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TransitionInfo {
    pub id: Uuid,
    pub transitioned_at: NaiveDateTime,
    pub changed_by: Option<Uuid>,
    pub reason: Option<String>,
}

// ==========================================
// BULK CSV IMPORT MODELS
// ==========================================

#[derive(Debug, serde::Deserialize)]
pub struct BulkImportCsvRow {
    pub primary_parent_first_name: String,
    pub primary_parent_last_name: String,
    pub primary_parent_email: String,
    pub primary_parent_phone: Option<String>,
    pub primary_parent_address: String,
    pub secondary_parent_first_name: Option<String>,
    pub secondary_parent_last_name: Option<String>,
    pub secondary_parent_email: Option<String>,
    pub secondary_parent_phone: Option<String>,
    pub child_first_name: String,
    pub child_last_name: String,
    pub child_gender: String,
    pub child_dob: Option<String>,
    pub classroom: String,
}

#[derive(Debug, Serialize)]
pub struct BulkImportRowError {
    pub row: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkImportResponse {
    pub created_families: usize,
    pub created_children: usize,
    pub row_errors: Vec<BulkImportRowError>,
}