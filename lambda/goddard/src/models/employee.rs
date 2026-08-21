use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};

// ─── Core DB structs ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Employee {
    pub id: Uuid,
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub employee_type: Option<String>,
    pub joined_on: Option<NaiveDate>,
    pub is_active: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmployeeWithUser {
    pub id: Uuid,
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub employee_type: Option<String>,
    pub joined_on: Option<NaiveDate>,
    pub is_active: Option<bool>,
    pub is_verified: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmployeeFormTemplate {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmployeeFormAssignment {
    pub id: Uuid,
    pub school_id: Uuid,
    pub employee_id: Uuid,
    pub user_id: Uuid,
    pub employee_form_template_id: Uuid,
    pub assignment_source: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub assigned_by: Option<Uuid>,
    pub assigned_at: Option<NaiveDateTime>,
    pub approved_by: Option<Uuid>,
    pub approved_on: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub is_active: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmployeeFormAssignmentWithTemplate {
    pub id: Uuid,
    pub school_id: Uuid,
    pub employee_id: Uuid,
    pub user_id: Uuid,
    pub employee_form_template_id: Uuid,
    pub form_name: String,
    pub fillout_form_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub assignment_source: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub assigned_by: Option<Uuid>,
    pub assigned_at: Option<NaiveDateTime>,
    pub approved_by: Option<Uuid>,
    pub approved_on: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub employee_first_name: Option<String>,
    pub employee_last_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmployeeFormSubmission {
    pub id: Uuid,
    pub school_id: Uuid,
    pub employee_id: Uuid,
    pub employee_form_assignment_id: Uuid,
    pub employee_form_template_id: Uuid,
    pub fillout_submission_id: String,
    pub form_data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub status: Option<String>,
    pub revision_number: Option<i32>,
    pub edit_link: Option<String>,
    pub pdf_link: Option<String>,
    pub submitted_at: Option<NaiveDateTime>,
    pub created_at: Option<NaiveDateTime>,
}

// ─── Request / Response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EmployeeInviteRequest {
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub employee_type: Option<String>,
    pub joined_on: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeInviteResponse {
    pub employee_id: Uuid,
    pub user_id: Uuid,
    pub invite_id: Uuid,
    pub email_sent: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkCreateEmployeesRequest {
    pub school_id: Uuid,
    pub employees: Vec<BulkEmployeeInput>,
}

#[derive(Debug, Deserialize)]
pub struct BulkEmployeeInput {
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct BulkCreatedEmployee {
    pub employee_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct BulkCreateEmployeesResponse {
    pub school_id: Uuid,
    pub created_count: usize,
    pub employees: Vec<BulkCreatedEmployee>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployeeRequest {
    pub phone: Option<String>,
    pub address: Option<String>,
    pub employee_type: Option<String>,
    pub joined_on: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployeeFormTemplateRequest {
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployeeFormTemplateRequest {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteEmployeeFormTemplateParams {
    pub form_id: Uuid,
    pub school_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AssignEmployeeFormRequest {
    pub employee_id: Uuid,
    pub employee_form_template_id: Uuid,
    pub school_id: Uuid,
    pub is_required: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewEmployeeFormRequest {
    pub assignment_id: Uuid,
    pub school_id: Uuid,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmployeeFormAssignmentQueryParams {
    pub school_id: Option<Uuid>,
    pub employee_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct EmployeeQueryParams {
    pub school_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteEmployeeFormAssignmentParams {
    pub assignment_id: Uuid,
    pub school_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkEmployeeFormReminderRequest {
    pub school_id: Uuid,
    pub reminders: Vec<EmployeeFormReminder>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmployeeFormReminder {
    pub employee_email: String,
    pub employee_name: String,
    pub form_name: String,
    pub due_date: String,
}

#[derive(Debug, Serialize)]
pub struct BulkEmployeeReminderResponse {
    pub total_sent: i32,
    pub total_failed: i32,
    pub failed_emails: Vec<String>,
    pub message: String,
}
