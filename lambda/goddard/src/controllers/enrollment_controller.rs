use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    services::enrollment_service::EnrollmentService,
    models::enrollment::{ParentInviteRequest, ResendConfirmationRequest},
    utils::ResponseUtils,
    error::AppError,
};

/// POST /enrollments/parent-invite
/// Create a parent invite for child enrollment (JWT protected - Admin/SuperAdmin)
pub async fn create_parent_invite(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Json(payload): Json<ParentInviteRequest>,
) -> Result<impl IntoResponse, AppError> {
    // JWT middleware will handle authentication and authorization
    // The service will validate business logic and permissions
    let response = enrollment_service.create_parent_invite(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// POST /enrollments/resend-confirmation
/// Resend parent confirmation email using Supabase auth ID (API Key protected)
pub async fn resend_parent_confirmation(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Json(payload): Json<ResendConfirmationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // API key middleware will handle authentication
    // parent_id is the Supabase auth user ID, not local users table ID
    let response = enrollment_service.resend_parent_confirmation(payload).await?;
    Ok(ResponseUtils::success(response))
}