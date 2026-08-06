use axum::{
    extract::{Path, Query, State},
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
    DeleteEmployeeFormAssignmentParams, DeleteEmployeeFormTemplateParams,
};
use crate::services::employee_service::EmployeeService;
use crate::middleware::auth::AuthContext;

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
