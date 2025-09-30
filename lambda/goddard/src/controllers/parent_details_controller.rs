use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    services::enrollment_service::EnrollmentService,
    utils::ResponseUtils,
    error::AppError,
};

/// GET /parent/{parent_id}
/// Get comprehensive details for a specific parent (API Key protected)
pub async fn get_parent_details_by_id(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(parent_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    println!("[DEBUG] GET Parent Details: Starting request for parent_id: {}", parent_id);

    // Extract API key from X-API-Key header for authentication
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] GET Parent Details: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    enrollment_service.validate_api_key(api_key).await?;
    println!("[DEBUG] GET Parent Details: Authentication successful");

    // Get parent details from service
    let response = enrollment_service.get_parent_details_by_id(parent_id).await?;

    println!("[DEBUG] GET Parent Details: Successfully retrieved parent details");
    Ok(ResponseUtils::success(response))
}