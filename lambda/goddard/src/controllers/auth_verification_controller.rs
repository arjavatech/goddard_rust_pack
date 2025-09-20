use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
    http::HeaderMap,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    services::{AuthService, auth_service::{ResendInvitationRequest, CreateInvitationRequest, CreateInvitationRequestEnhanced}},
    utils::ResponseUtils,
    error::AppError,
};

#[derive(Debug, Deserialize)]
pub struct VerificationQuery {
    pub school_id: Option<String>,
    pub include_details: Option<bool>,
}


/// GET /auth/verification-status
/// Returns the current authorization verification status for all users
pub async fn get_auth_verification_status(
    State(auth_service): State<Arc<AuthService>>,
    Query(params): Query<VerificationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let response = auth_service
        .get_auth_verification_status(params.school_id, params.include_details)
        .await?;

    Ok(ResponseUtils::success(response))
}

/// GET /auth/invitation-summary
/// Returns a summary of invitation statuses grouped by status type
pub async fn get_invitation_summary(
    State(auth_service): State<Arc<AuthService>>,
) -> Result<impl IntoResponse, AppError> {
    let summary = auth_service.get_invitation_summary().await?;
    Ok(ResponseUtils::success(summary))
}

/// POST /auth/resend-invitation
/// Endpoint to resend invitation email to users who haven't confirmed
pub async fn resend_invitation(
    State(auth_service): State<Arc<AuthService>>,
    Json(payload): Json<ResendInvitationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = auth_service.resend_invitation(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// POST /auth/invite-create
/// Endpoint to create a new user invitation
pub async fn create_invitation(
    State(auth_service): State<Arc<AuthService>>,
    Json(payload): Json<CreateInvitationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = auth_service.create_invitation(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// POST /auth/invite-create-enhanced
/// Endpoint to create a new user invitation with custom fields (school_id, first_name, last_name, role)
pub async fn create_invitation_enhanced(
    State(auth_service): State<Arc<AuthService>>,
    Json(payload): Json<CreateInvitationRequestEnhanced>,
) -> Result<impl IntoResponse, AppError> {
    let response = auth_service.create_invitation_enhanced(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// DELETE /auth/clear-table
/// Clear Supabase authentication table (Admin only)
pub async fn clear_auth_table(
    headers: HeaderMap,
    State(auth_service): State<Arc<AuthService>>,
) -> Result<impl IntoResponse, AppError> {
    // Check for admin API key
    if let Some(api_key) = headers.get("X-API-Key").and_then(|v| v.to_str().ok()) {
        if api_key != std::env::var("OWNER_API_KEY").unwrap_or_default() {
            return Err(AppError::Authorization("Invalid API key".to_string()));
        }
    } else {
        return Err(AppError::Authorization("API key required".to_string()));
    }

    auth_service.clear_auth_table().await?;
    Ok(ResponseUtils::success_with_message("Authentication table cleared successfully"))
}

/// GET /auth/debug-users
/// Debug endpoint to list all Supabase auth users with metadata (Admin only)
pub async fn debug_auth_users(
    headers: HeaderMap,
    State(auth_service): State<Arc<AuthService>>,
) -> Result<impl IntoResponse, AppError> {
    // Check for admin API key
    if let Some(api_key) = headers.get("X-API-Key").and_then(|v| v.to_str().ok()) {
        if api_key != std::env::var("OWNER_API_KEY").unwrap_or_default() {
            return Err(AppError::Authorization("Invalid API key".to_string()));
        }
    } else {
        return Err(AppError::Authorization("API key required".to_string()));
    }

    let users = auth_service.debug_list_auth_users().await?;
    Ok(ResponseUtils::success(users))
}

#[derive(Debug, Deserialize)]
pub struct UserFilterQuery {
    pub school_id: String,
    pub role: String,
}

/// GET /auth/users/filter
/// Get users filtered by school_id and role with specific response format
pub async fn get_users_by_school_and_role(
    State(auth_service): State<Arc<AuthService>>,
    Query(params): Query<UserFilterQuery>,
) -> Result<impl IntoResponse, AppError> {
    let users = auth_service
        .get_users_by_school_and_role(&params.school_id, &params.role)
        .await?;

    Ok(ResponseUtils::success(users))
}

/// GET /users/me
/// Get current user profile from JWT token
pub async fn get_current_user_profile(
    headers: HeaderMap,
    State(auth_service): State<Arc<AuthService>>,
) -> Result<impl IntoResponse, AppError> {
    // Extract JWT token from Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Authorization("Missing Authorization header".to_string()))?;

    // Check if it starts with "Bearer "
    let jwt_token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..] // Remove "Bearer " prefix
    } else {
        auth_header // Use as-is if no Bearer prefix
    };

    let user_profile = auth_service
        .get_user_profile_from_jwt(jwt_token)
        .await?;

    Ok(ResponseUtils::success(user_profile))
}