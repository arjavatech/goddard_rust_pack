use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::form_submission::{
    CreateFormSubmissionWebhookRequest, FormSubmissionResponse,
    FormSubmissionVersionResponse, UpdateFormSubmissionStatusRequest,
};
use crate::services::FormSubmissionService;

#[derive(Deserialize)]
pub struct FormSubmissionQuery {
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub form_template_id: Uuid,
}

// Create Form Submission (Webhook from Fillout)
pub async fn create_form_submission_webhook(
    State(service): State<Arc<FormSubmissionService>>,
    headers: HeaderMap,
    Json(request): Json<CreateFormSubmissionWebhookRequest>,
) -> Result<(StatusCode, Json<FormSubmissionResponse>), AppError> {

    // Extract webhook secret from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;


    // Validate webhook secret
    service.validate_webhook_secret(api_key).await?;

    // Create form submission with enhanced error handling
    match service.create_form_submission_from_webhook(request).await {
        Ok(submission) => {
            Ok((StatusCode::CREATED, Json(submission)))
        }
        Err(e) => {
            println!("[ERROR] Failed to create form submission: {:?}", e);
            Err(e)
        }
    }
}

// Get Latest Form Submission (Most Recent Version)
pub async fn get_latest_form_submission(
    State(service): State<Arc<FormSubmissionService>>,
    headers: HeaderMap,
    Query(query): Query<FormSubmissionQuery>,
) -> Result<Json<Option<FormSubmissionResponse>>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] GET Latest: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_webhook_secret(api_key).await?;

    let submission = service
        .get_latest_form_submission(
            query.school_id,
            query.enrollment_id,
            query.form_template_id,
        )
        .await?;

    Ok(Json(submission))
}

// Get All Form Submission Versions (Version History)
pub async fn get_form_submission_versions(
    State(service): State<Arc<FormSubmissionService>>,
    headers: HeaderMap,
    Query(query): Query<FormSubmissionQuery>,
) -> Result<Json<Vec<FormSubmissionVersionResponse>>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] GET Versions: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_webhook_secret(api_key).await?;

    let versions = service
        .get_all_form_submission_versions(
            query.school_id,
            query.enrollment_id,
            query.form_template_id,
        )
        .await?;

    Ok(Json(versions))
}

// Get Form Submission by ID
pub async fn get_form_submission_by_id(
    State(service): State<Arc<FormSubmissionService>>,
    headers: HeaderMap,
    Path(submission_id): Path<Uuid>,
) -> Result<Json<FormSubmissionResponse>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] GET ByID: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_webhook_secret(api_key).await?;

    let submission = service.get_form_submission_by_id(submission_id).await?;

    Ok(Json(submission))
}

#[derive(Serialize)]
pub struct ResumeLinkResponse {
    pub edit_link: Option<String>,
}

// Get Resume Link for In-Progress Form (parent or admin)
pub async fn get_form_resume_link(
    State(service): State<Arc<FormSubmissionService>>,
    Path(assignment_id): Path<Uuid>,
) -> Result<Json<ResumeLinkResponse>, AppError> {

    let edit_link = service.get_form_resume_link(assignment_id).await?;

    Ok(Json(ResumeLinkResponse { edit_link }))
}

// Update Form Submission Status (Admin/SuperAdmin only)
pub async fn update_form_submission_status(
    State(service): State<Arc<FormSubmissionService>>,
    headers: HeaderMap,
    Path(submission_id): Path<Uuid>,
    Json(request): Json<UpdateFormSubmissionStatusRequest>,
) -> Result<Json<FormSubmissionResponse>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] PUT Status: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_webhook_secret(api_key).await?;

    let submission = service
        .update_form_submission_status(submission_id, request)
        .await?;

    Ok(Json(submission))
}