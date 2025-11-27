use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
    http::HeaderMap,
};
use serde::Deserialize;
use std::sync::Arc;
use serde_json::json;
use axum::http::StatusCode;

use crate::{
    services::{AuthService, auth_service::{ResendInvitationRequest, CreateInvitationRequest, CreateInvitationRequestEnhanced, UpdateAdminRequest, DeleteAdminRequest}},
    utils::ResponseUtils,
    error::AppError,
    middleware::auth::AuthContext,
};
use axum::Extension;

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

#[derive(Debug, Deserialize)]
pub struct GetAdminsBySchoolQuery {
    pub school_id: String,
}

/// GET /users/admin?school_id=<uuid>
/// Get all verified Admin users for a specific school (SuperAdmin only)
pub async fn get_admins_by_school(
    State(auth_service): State<Arc<AuthService>>,
    Query(query): Query<GetAdminsBySchoolQuery>,
) -> impl IntoResponse {
    match auth_service.get_admins_by_school(&query.school_id).await {
        Ok(admins) => {
            let count = admins.len();
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": admins,
                    "count": count,
                    "timestamp": chrono::Utc::now()
                }))
            ).into_response()
        },
        Err(e) => e.into_response(),
    }
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

/// PUT /users/admin - Update OWN admin profile (Admin + SuperAdmin)
/// Admin can only update their own profile, identified from JWT
pub async fn update_admin_user(
    Extension(auth): Extension<AuthContext>,
    State(auth_service): State<Arc<AuthService>>,
    Json(payload): Json<UpdateAdminRequest>,
) -> impl IntoResponse {
    match auth_service.update_admin_user(auth.user_id, payload).await {
        Ok(admin) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Admin user updated successfully",
                "data": admin,
                "timestamp": chrono::Utc::now()
            }))
        ).into_response(),
        Err(e) => e.into_response(),
    }
}

/// DELETE /users/admin - Soft delete admin user (SuperAdmin only)
pub async fn delete_admin_user(
    State(auth_service): State<Arc<AuthService>>,
    Json(payload): Json<DeleteAdminRequest>,
) -> impl IntoResponse {
    match auth_service.delete_admin_user(payload).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Admin user deleted successfully",
                "timestamp": chrono::Utc::now()
            }))
        ).into_response(),
        Err(e) => e.into_response(),
    }
}