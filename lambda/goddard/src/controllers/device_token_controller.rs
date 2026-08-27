use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    dao::DeviceTokenDao, error::AppError, middleware::auth::AuthContext, utils::ResponseUtils,
};

#[derive(Deserialize)]
pub struct RegisterDeviceTokenRequest {
    pub token: String,
    #[serde(default)]
    pub platform: Option<String>,
}

#[derive(Deserialize)]
pub struct UnregisterDeviceTokenRequest {
    pub token: String,
}

/// POST /device-tokens
pub async fn register_device_token(
    Extension(auth): Extension<AuthContext>,
    State(dao): State<Arc<DeviceTokenDao>>,
    headers: HeaderMap,
    Json(body): Json<RegisterDeviceTokenRequest>,
) -> Result<StatusCode, AppError> {
    let token = body.token.trim();
    if token.is_empty() {
        return Err(AppError::Validation("token is required".to_string()));
    }

    let platform = body
        .platform
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("web");

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    dao.upsert_token(auth.user_id, token, platform, user_agent)
        .await?;

    println!(
        "[DeviceTokenController] upserted token (user={}, platform={})",
        auth.user_id, platform
    );

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /device-tokens
///
/// Tokens are bearer credentials and must not be included in a URL, where they
/// can be retained in browser history, proxies, or access logs.
pub async fn unregister_device_token(
    Extension(auth): Extension<AuthContext>,
    State(dao): State<Arc<DeviceTokenDao>>,
    Json(body): Json<UnregisterDeviceTokenRequest>,
) -> Result<StatusCode, AppError> {
    let token = body.token.trim();
    if token.is_empty() {
        return Err(AppError::Validation("token is required".to_string()));
    }
    dao.delete_token_for_user(token, auth.user_id).await?;

    println!(
        "[DeviceTokenController] deleted token (user={})",
        auth.user_id
    );

    Ok(StatusCode::NO_CONTENT)
}

/// GET /device-tokens/status
///
/// Returns only registration metadata for the authenticated user. FCM tokens
/// are never exposed to a browser after registration.
pub async fn device_token_status(
    Extension(auth): Extension<AuthContext>,
    State(dao): State<Arc<DeviceTokenDao>>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(ResponseUtils::success(
        dao.status_for_user(auth.user_id).await?,
    ))
}
