use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::error::error_types::AppError;
use crate::models::class_form_override::{CreateClassFormOverrideRequest, ClassFormOverrideResponse, DeleteClassFormOverrideParams, DeleteClassFormOverrideResponse};
use crate::services::class_form_override_service::ClassFormOverrideService;

pub async fn create_class_form_override(
    State(class_form_override_service): State<Arc<ClassFormOverrideService>>,
    Json(payload): Json<CreateClassFormOverrideRequest>,
) -> Result<impl IntoResponse, AppError> {
    let override_obj = class_form_override_service.create_class_form_override(payload).await?;
    let response = ClassFormOverrideResponse::from(override_obj);

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_class_form_override(
    State(class_form_override_service): State<Arc<ClassFormOverrideService>>,
    Query(params): Query<DeleteClassFormOverrideParams>,
) -> Result<impl IntoResponse, AppError> {
    let deleted_override = class_form_override_service.delete_class_form_override(params.id).await?;

    let response = DeleteClassFormOverrideResponse {
        message: "Class form override successfully deleted".to_string(),
        id: deleted_override.id,
        school_id: deleted_override.school_id,
        classroom_id: deleted_override.classroom_id,
        form_template_id: deleted_override.form_template_id,
    };

    Ok((StatusCode::OK, Json(response)))
}