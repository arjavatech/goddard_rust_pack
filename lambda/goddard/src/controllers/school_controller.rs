use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;
use std::sync::Arc;

use crate::{
    services::SchoolService,
    models::school::{CreateSchoolRequest, UpdateSchoolRequest},
    utils::ResponseUtils,
    error::AppError,
};

/// POST /schools
/// Create a new school (API Key protected - SuperAdmin only)
pub async fn create_school(
    State(school_service): State<Arc<SchoolService>>,
    Json(payload): Json<CreateSchoolRequest>,
) -> Result<impl IntoResponse, AppError> {
    // This endpoint is protected by API key middleware, no need for JWT auth
    let response = school_service.create_school(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /schools
/// Get all schools (Public endpoint)
pub async fn get_all_schools(
    State(school_service): State<Arc<SchoolService>>,
) -> Result<impl IntoResponse, AppError> {
    let response = school_service.get_all_schools().await?;
    Ok(ResponseUtils::success(response))
}

/// PUT /schools
/// Update a school (API Key protected - SuperAdmin only)
pub async fn update_school(
    State(school_service): State<Arc<SchoolService>>,
    Json(payload): Json<UpdateSchoolRequest>,
) -> Result<impl IntoResponse, AppError> {
    // This endpoint is protected by API key middleware, no need for JWT auth
    let response = school_service.update_school(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// DELETE /schools/:id
/// Delete a school (API Key protected - SuperAdmin only)
pub async fn delete_school(
    State(school_service): State<Arc<SchoolService>>,
    Path(school_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // This endpoint is protected by API key middleware, no need for JWT auth
    let response = school_service.delete_school(&school_id).await?;
    Ok(ResponseUtils::success(response))
}