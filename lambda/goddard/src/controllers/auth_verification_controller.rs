use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
    http::HeaderMap,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    services::{AuthService, auth_service::{ResendInvitationRequest, CreateInvitationRequest}},
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