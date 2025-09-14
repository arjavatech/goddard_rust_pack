use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate};
use uuid::Uuid;
use std::collections::HashMap;

// ============================================================================
// Core Domain Models
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct School {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
    pub settings: HashMap<String, serde_json::Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub school_id: Uuid,
    pub email: String,
    pub role: UserRole,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub is_active: bool,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Parent,
    Teacher,
    Admin,
    SuperAdmin,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Child {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: NaiveDate,
    pub age_group: AgeGroup,
    pub medical_info: Option<MedicalInfo>,
    pub enrollment_status: EnrollmentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AgeGroup {
    Infant,
    Toddler,
    Preschool,
    PreK,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MedicalInfo {
    pub allergies: Vec<String>,
    pub medications: Vec<String>,
    pub special_needs: Option<String>,
    pub emergency_medical_info: Option<String>,
    pub pediatrician: Option<Pediatrician>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pediatrician {
    pub name: String,
    pub phone: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    Pending,
    Enrolled,
    Graduated,
    Withdrawn,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Classroom {
    pub id: Uuid,
    pub school_id: Uuid,
    pub name: String,
    pub age_group: AgeGroup,
    pub capacity: i32,
    pub enrolled_count: i32,
    pub available_spots: i32,
    pub teachers: Vec<ClassroomTeacher>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClassroomTeacher {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub role: TeacherRole,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TeacherRole {
    LeadTeacher,
    AssistantTeacher,
    Substitute,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Enrollment {
    pub id: Uuid,
    pub child_id: Uuid,
    pub school_id: Uuid,
    pub classroom_id: Option<Uuid>,
    pub status: EnrollmentWorkflowStatus,
    pub admin_approval_status: AdminApprovalStatus,
    pub progress: Option<EnrollmentProgress>,
    pub start_date: Option<NaiveDate>,
    pub child: Option<Child>,
    pub parent: Option<User>,
    pub classroom: Option<Classroom>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentWorkflowStatus {
    Pending,
    InProgress,
    UnderReview,
    Approved,
    Rejected,
    NeedsRevision,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AdminApprovalStatus {
    Pending,
    Approved,
    Rejected,
    NeedsRevision,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrollmentProgress {
    pub total_forms: i32,
    pub completed_forms: i32,
    pub pending_forms: i32,
    pub completion_percentage: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormTemplate {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_description: Option<String>,
    pub form_type: FormType,
    pub fillout_form_id: String,
    pub fillout_form_url: String,
    pub status: FormStatus,
    pub is_required: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FormType {
    Admission,
    Medical,
    Emergency,
    Authorization,
    Handbook,
    Agreement,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FormStatus {
    Active,
    SchoolDefault,
    Draft,
    Archive,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormSubmission {
    pub id: Uuid,
    pub enrollment_id: Uuid,
    pub form_template_id: Uuid,
    pub fillout_submission_id: String,
    pub form_data: HashMap<String, serde_json::Value>,
    pub status: SubmissionStatus,
    pub submitted_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Submitted,
    Locked,
    RevisionNeeded,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailNotification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email_address: String,
    pub email_type: EmailType,
    pub is_active: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EmailType {
    Primary,
    Work,
    Backup,
    Emergency,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub enrollment_id: Uuid,
    pub document_name: String,
    pub document_type: DocumentType,
    pub file_url: String,
    pub file_size: i64,
    pub mime_type: String,
    pub is_verified: bool,
    pub uploaded_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Immunization,
    BirthCertificate,
    Insurance,
    PhotoRelease,
    MedicalForm,
    Other,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserRequest {
    pub email: String,
    pub role: UserRole,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub send_welcome_email: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateChildRequest {
    pub first_name: String,
    pub last_name: String,
    pub birth_date: NaiveDate,
    pub medical_info: Option<MedicalInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateClassroomRequest {
    pub name: String,
    pub age_group: AgeGroup,
    pub capacity: i32,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateEnrollmentRequest {
    pub child_id: Uuid,
    pub classroom_id: Uuid,
    pub start_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEnrollmentRequest {
    pub status: Option<EnrollmentWorkflowStatus>,
    pub admin_notes: Option<String>,
    pub notify_parent: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApproveEnrollmentRequest {
    pub approval_notes: Option<String>,
    pub notify_parents: Option<bool>,
    pub lock_forms: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RejectEnrollmentRequest {
    pub rejection_notes: Option<String>,
    pub notify_parents: Option<bool>,
    pub unlock_forms: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateFormTemplateRequest {
    pub form_name: String,
    pub form_description: Option<String>,
    pub form_type: FormType,
    pub fillout_form_id: String,
    pub fillout_form_url: String,
    pub is_required: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FilloutWebhookRequest {
    pub submissionId: String,
    pub formId: String,
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateEmailNotificationRequest {
    pub email_address: String,
    pub email_type: EmailType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentRequest {
    pub enrollment_id: Uuid,
    pub document_name: String,
    pub document_type: DocumentType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DashboardOverview {
    pub total_enrollments: i32,
    pub pending_approvals: i32,
    pub completed_enrollments: i32,
    pub total_children: i32,
    pub total_parents: i32,
    pub form_completion_rate: f64,
}

// ============================================================================
// Pagination and Error Models
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
    pub links: PaginationLinks,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaginationMeta {
    pub page: i32,
    pub per_page: i32,
    pub total: i32,
    pub total_pages: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaginationLinks {
    pub self_link: String,
    pub next: Option<String>,
    pub prev: Option<String>,
    pub first: String,
    pub last: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiError {
    pub error_type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub instance: Option<String>,
    pub school_id: Option<Uuid>,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub errors: Option<Vec<FieldError>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryParams {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub sort: Option<String>,
    pub role: Option<UserRole>,
    pub parent_id: Option<Uuid>,
    pub status: Option<EnrollmentWorkflowStatus>,
    pub form_type: Option<FormType>,
    pub enrollment_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
}