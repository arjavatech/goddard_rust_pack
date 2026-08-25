use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
#[allow(unused_imports)]
use axum::Extension;

use crate::error::error_types::AppError;
use crate::models::employee::{
    EmployeeInviteRequest, UpdateEmployeeRequest, CreateEmployeeFormTemplateRequest,
    UpdateEmployeeFormTemplateRequest, AssignEmployeeFormRequest, ReviewEmployeeFormRequest,
    BulkEmployeeFormReminderRequest, EmployeeFormAssignmentQueryParams, EmployeeQueryParams,
    BulkCreateEmployeesRequest, BulkEmployeeInput,
    AssignEmployeeFormToSchoolRequest,
    ResendEmployeeInviteRequest,
    DeleteEmployeeFormAssignmentParams, DeleteEmployeeFormTemplateParams,
};
use crate::models::form_review_queue::{EmployeeFormReviewQueueItem, FormReviewQueueQuery};
use crate::services::employee_service::EmployeeService;
use crate::middleware::auth::{AuthContext, check_permission_admin_or_superadmin};

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SchoolIdQuery {
    pub school_id: Option<Uuid>,
}

// ─── Simple response shapes ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ─── Employee handlers ───────────────────────────────────────────────────────

pub async fn invite_employee(
    State(svc): State<Arc<EmployeeService>>,
    Json(payload): Json<EmployeeInviteRequest>,
) -> Result<impl IntoResponse, AppError> {
    let resp = svc.invite_employee(payload).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn bulk_create_employees(
    State(svc): State<Arc<EmployeeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut school_id = None;
    let mut csv_bytes = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::Validation(format!("Failed to read multipart field: {}", e)))?
    {
        match field.name() {
            Some("school_id") => {
                let value = field.text().await
                    .map_err(|e| AppError::Validation(format!("Failed to read school_id: {}", e)))?;
                school_id = Some(Uuid::parse_str(value.trim())
                    .map_err(|_| AppError::Validation("school_id must be a valid UUID".to_string()))?);
            }
            Some("file") => {
                csv_bytes = Some(field.bytes().await
                    .map_err(|e| AppError::Validation(format!("Failed to read CSV file: {}", e)))?
                    .to_vec());
            }
            _ => {}
        }
    }

    let school_id = school_id
        .ok_or_else(|| AppError::Validation("Missing required field: school_id".to_string()))?;
    let csv_bytes = csv_bytes
        .ok_or_else(|| AppError::Validation("Missing required field: file".to_string()))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_bytes.as_slice());
    let mut employees = Vec::new();
    for (index, row) in reader.deserialize::<BulkEmployeeInput>().enumerate() {
        employees.push(row.map_err(|e| {
            AppError::Validation(format!("CSV row {} is invalid: {}", index + 2, e))
        })?);
    }

    let payload = BulkCreateEmployeesRequest { school_id, employees };
    check_permission_admin_or_superadmin(&auth, &payload.school_id)?;
    let response = svc.bulk_create_employees(payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}
pub async fn resend_employee_invite(State(svc): State<Arc<EmployeeService>>, Path(employee_id): Path<Uuid>, axum::Extension(auth): axum::Extension<AuthContext>, Json(payload): Json<ResendEmployeeInviteRequest>) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &payload.school_id)?;
    Ok((StatusCode::OK, Json(svc.resend_employee_invite(employee_id, payload.school_id).await?)))
}

pub async fn get_employees(
    State(svc): State<Arc<EmployeeService>>,
    Query(params): Query<EmployeeQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    let employees = svc.get_employees(school_id).await?;
    Ok((StatusCode::OK, Json(employees)))
}

pub async fn get_employee_by_id(
    State(svc): State<Arc<EmployeeService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    let employee = svc.get_employee_by_id(employee_id, school_id).await?;
    Ok((StatusCode::OK, Json(employee)))
}

pub async fn update_employee(
    State(svc): State<Arc<EmployeeService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
    Json(payload): Json<UpdateEmployeeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    let employee = svc.update_employee(employee_id, school_id, payload).await?;
    Ok((StatusCode::OK, Json(employee)))
}

pub async fn deactivate_employee(
    State(svc): State<Arc<EmployeeService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    svc.deactivate_employee(employee_id, school_id).await?;
    Ok((StatusCode::OK, Json(MessageResponse { message: "Employee deactivated successfully".to_string() })))
}

pub async fn activate_employee(
    State(svc): State<Arc<EmployeeService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    svc.activate_employee(employee_id, school_id).await?;
    Ok((StatusCode::OK, Json(MessageResponse { message: "Employee activated successfully".to_string() })))
}

pub async fn get_current_employee(
    State(svc): State<Arc<EmployeeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    let employee = svc.get_current_employee(auth.user_id, school_id).await?;
    Ok((StatusCode::OK, Json(employee)))
}

pub async fn get_employee_forms(
    State(svc): State<Arc<EmployeeService>>,
    Path(employee_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let assignments = svc.get_assignments_by_employee(employee_id).await?;
    Ok((StatusCode::OK, Json(assignments)))
}

// ─── Employee Form Template handlers ────────────────────────────────────────

pub async fn create_employee_form_template(
    State(svc): State<Arc<EmployeeService>>,
    Json(payload): Json<CreateEmployeeFormTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let template = svc.create_form_template(payload).await?;
    Ok((StatusCode::CREATED, Json(template)))
}

pub async fn get_employee_form_templates(
    State(svc): State<Arc<EmployeeService>>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("school_id is required".to_string()))?;
    let templates = svc.get_form_templates(school_id).await?;
    Ok((StatusCode::OK, Json(templates)))
}

pub async fn update_employee_form_template(
    State(svc): State<Arc<EmployeeService>>,
    Json(payload): Json<UpdateEmployeeFormTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let template = svc.update_form_template(payload).await?;
    Ok((StatusCode::OK, Json(template)))
}

pub async fn delete_employee_form_template(
    State(svc): State<Arc<EmployeeService>>,
    Query(params): Query<DeleteEmployeeFormTemplateParams>,
) -> Result<impl IntoResponse, AppError> {
    svc.delete_form_template(params.form_id, params.school_id).await?;
    Ok((StatusCode::OK, Json(MessageResponse { message: "Employee form template deleted successfully".to_string() })))
}

// ─── Employee Form Assignment handlers ──────────────────────────────────────

pub async fn assign_employee_form(
    State(svc): State<Arc<EmployeeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(payload): Json<AssignEmployeeFormRequest>,
) -> Result<impl IntoResponse, AppError> {
    let assigned_by = auth.user_id;
    let assignment = svc.assign_form(payload, assigned_by).await?;
    Ok((StatusCode::CREATED, Json(assignment)))
}

pub async fn assign_employee_form_to_school(
    State(svc): State<Arc<EmployeeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(payload): Json<AssignEmployeeFormToSchoolRequest>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &payload.school_id)?;
    let response = svc.assign_form_to_all_employees(payload, auth.user_id).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_employee_form_assignments(
    State(svc): State<Arc<EmployeeService>>,
    Query(params): Query<EmployeeFormAssignmentQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(employee_id) = params.employee_id {
        let assignments = svc.get_assignments_by_employee(employee_id).await?;
        return Ok((StatusCode::OK, Json(serde_json::to_value(assignments).unwrap_or_default())));
    }
    if let Some(school_id) = params.school_id {
        let assignments = svc.get_assignments_by_school(school_id).await?;
        return Ok((StatusCode::OK, Json(serde_json::to_value(assignments).unwrap_or_default())));
    }
    Err(AppError::Validation("Either school_id or employee_id is required".to_string()))
}

pub async fn get_employee_form_review_queue(
    State(svc): State<Arc<EmployeeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Query(query): Query<FormReviewQueueQuery>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &query.school_id)?;
    let items: Vec<EmployeeFormReviewQueueItem> = svc.get_review_queue(&query).await?;
    Ok((StatusCode::OK, Json(items)))
}

pub async fn review_employee_form_assignment(
    State(svc): State<Arc<EmployeeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(payload): Json<ReviewEmployeeFormRequest>,
) -> Result<impl IntoResponse, AppError> {
    let reviewer_id = auth.user_id;
    let assignment = svc.review_assignment(payload, reviewer_id).await?;
    Ok((StatusCode::OK, Json(assignment)))
}

pub async fn delete_employee_form_assignment(
    State(svc): State<Arc<EmployeeService>>,
    Query(params): Query<DeleteEmployeeFormAssignmentParams>,
) -> Result<impl IntoResponse, AppError> {
    svc.delete_assignment(params.assignment_id, params.school_id).await?;
    Ok((StatusCode::OK, Json(MessageResponse { message: "Employee form assignment deleted successfully".to_string() })))
}

// ─── Employee Form Submission webhook ───────────────────────────────────────

#[derive(Deserialize)]
pub struct EmployeeFormWebhookPayload {
    pub employee_form_assignment_id: Uuid,
    pub fillout_submission_id: String,
    pub form_data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub edit_link: Option<String>,
    pub pdf_link: Option<String>,
    /// Fillout's original submission instant. The backend converts this to the
    /// owning school's configured local wall-clock time before persisting it.
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn employee_form_submission_webhook(
    State(svc): State<Arc<EmployeeService>>,
    headers: HeaderMap,
    Json(payload): Json<EmployeeFormWebhookPayload>,
) -> Result<impl IntoResponse, AppError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Authentication("Missing X-API-Key header".to_string()))?;

    let expected = std::env::var("OWNER_API_KEY")
        .map_err(|_| AppError::Internal("OWNER_API_KEY not configured".to_string()))?;

    if api_key != expected {
        return Err(AppError::Authentication("Invalid API key".to_string()));
    }

    let submission = svc.handle_form_webhook(
        payload.employee_form_assignment_id,
        &payload.fillout_submission_id,
        payload.form_data.as_ref(),
        payload.metadata.as_ref(),
        payload.edit_link.as_deref(),
        payload.pdf_link.as_deref(),
        payload.submitted_at,
    ).await?;
    Ok((StatusCode::OK, Json(submission)))
}

// ─── Bulk reminders ─────────────────────────────────────────────────────────

pub async fn send_bulk_employee_form_reminders(
    State(svc): State<Arc<EmployeeService>>,
    Json(payload): Json<BulkEmployeeFormReminderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let result = svc.send_bulk_reminders(payload).await?;
    Ok((StatusCode::OK, Json(result)))
}
