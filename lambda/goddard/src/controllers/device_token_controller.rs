use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    dao::DeviceTokenDao,
    error::AppError,
    middleware::auth::AuthContext,
};

#[derive(Deserialize)]
pub struct RegisterDeviceTokenRequest {
    pub token: String,
    #[serde(default)]
    pub platform: Option<String>,
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

/// DELETE /device-tokens/:token
pub async fn unregister_device_token(
    Extension(auth): Extension<AuthContext>,
    State(dao): State<Arc<DeviceTokenDao>>,
    Path(token): Path<String>,
) -> Result<StatusCode, AppError> {
    dao.delete_token_for_user(&token, auth.user_id).await?;

    println!(
        "[DeviceTokenController] deleted token (user={})",
        auth.user_id
    );

    Ok(StatusCode::NO_CONTENT)
}
