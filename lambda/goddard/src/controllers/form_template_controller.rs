use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::error_types::AppError;
use crate::models::form_template::{CreateFormTemplateRequest, UpdateFormTemplateRequest, FormTemplateResponse, FormTemplateListResponse, DeleteFormTemplateParams};
use crate::services::form_template_service::FormTemplateService;

#[derive(Deserialize)]
pub struct FormTemplateQueryParams {
    pub school_id: Option<Uuid>,
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
    let response = FormTemplateResponse {
        id: form_template.id,
        school_id: form_template.school_id,
        form_name: form_template.form_name,
        form_type: form_template.form_type,
        fillout_form_id: form_template.fillout_form_id,
        fillout_form_url: form_template.fillout_form_url,
        status: form_template.status,
        is_required: form_template.is_required,
        display_order: form_template.display_order,
        created_at: form_template.created_at,
        updated_at: Some(form_template.updated_at),
    };

    Ok((StatusCode::OK, Json(response)))
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