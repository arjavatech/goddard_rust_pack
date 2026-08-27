use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::error_types::AppError;
use crate::models::form_template::{CreateFormTemplateRequest, UpdateFormTemplateRequest, FormTemplateResponse, FormTemplateListResponse, DeleteFormTemplateParams};
use crate::models::document_request::{UploadIntentRequest, UploadIntentResponse, CompleteUploadRequest, FileAccessResponse};
use crate::services::form_template_service::FormTemplateService;
use crate::middleware::auth::{AuthContext, check_permission_admin_or_superadmin};
use axum::Extension;

#[derive(Deserialize)]
pub struct FormTemplateQueryParams {
    pub school_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct FormTemplatePdfQueryParams {
    pub school_id: Option<Uuid>,
    pub download: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct DeleteFormTemplateResponse {
    pub message: String,
    pub form_id: Uuid,
    pub school_id: Uuid,
}

pub async fn create_form_template(
    State(form_template_service): State<Arc<FormTemplateService>>,
    Json(payload): Json<CreateFormTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let form_template = form_template_service.create_form_template(payload).await?;
    let response = FormTemplateResponse::from(form_template);

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_form_templates_by_school(
    State(form_template_service): State<Arc<FormTemplateService>>,
    Query(params): Query<FormTemplateQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("Missing required parameter: school_id".to_string()))?;

    tracing::info!("Fetching form templates for school_id: {}", school_id);

    match form_template_service.get_form_templates_by_school(school_id).await {
        Ok(form_templates) => {
            let response: Vec<FormTemplateListResponse> = form_templates
                .into_iter()
                .map(FormTemplateListResponse::from)
                .collect();

            tracing::info!("Successfully fetched {} form templates", response.len());
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            tracing::error!("Database error: {:?}", e);
            Err(AppError::Database(format!("Failed to fetch form templates for school {}: {}", school_id, e)))
        }
    }
}

pub async fn update_form_template(
    State(form_template_service): State<Arc<FormTemplateService>>,
    Json(payload): Json<UpdateFormTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let form_template = form_template_service.update_form_template(payload).await?;
    let response = FormTemplateResponse::from(form_template);

    Ok((StatusCode::OK, Json(response)))
}

pub async fn form_template_pdf_upload_intent(
    State(service): State<Arc<FormTemplateService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<FormTemplatePdfQueryParams>,
    Json(body): Json<UploadIntentRequest>,
) -> Result<Json<UploadIntentResponse>, AppError> {
    let school_id = query.school_id.ok_or_else(|| AppError::Validation("school_id is required".into()))?;
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    Ok(Json(service.pdf_upload_intent(id, school_id, &body).await?))
}

pub async fn complete_form_template_pdf_upload(
    State(service): State<Arc<FormTemplateService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<FormTemplatePdfQueryParams>,
    Json(body): Json<CompleteUploadRequest>,
) -> Result<Json<FormTemplateResponse>, AppError> {
    let school_id = query.school_id.ok_or_else(|| AppError::Validation("school_id is required".into()))?;
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    Ok(Json(FormTemplateResponse::from(service.complete_pdf_upload(id, school_id, &body).await?)))
}

pub async fn form_template_pdf_url(
    State(service): State<Arc<FormTemplateService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<FormTemplatePdfQueryParams>,
) -> Result<Json<FileAccessResponse>, AppError> {
    let school_id = query.school_id.ok_or_else(|| AppError::Validation("school_id is required".into()))?;
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    Ok(Json(service.pdf_access_url(id, school_id, query.download.unwrap_or(false)).await?))
}

pub async fn remove_form_template_pdf(
    State(service): State<Arc<FormTemplateService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<FormTemplatePdfQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = query.school_id.ok_or_else(|| AppError::Validation("school_id is required".into()))?;
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    service.remove_pdf(id, school_id).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({"message":"PDF template removed"}))))
}

pub async fn delete_form_template(
    State(form_template_service): State<Arc<FormTemplateService>>,
    Query(params): Query<DeleteFormTemplateParams>,
) -> Result<impl IntoResponse, AppError> {
    form_template_service.delete_form_template(params.form_id, params.school_id).await?;

    let response = DeleteFormTemplateResponse {
        message: "Form template successfully deleted".to_string(),
        form_id: params.form_id,
        school_id: params.school_id,
    };

    Ok((StatusCode::OK, Json(response)))
}
