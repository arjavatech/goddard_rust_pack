use std::sync::Arc;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthContext;
use crate::services::owner_service::{
    InviteOwnerRequest, ResendOwnerInviteRequest, UpdateOwnerRequest, OwnerService,
};

pub async fn invite_owner(
    State(svc): State<Arc<OwnerService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<InviteOwnerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let result = svc.invite_owner(payload, &auth).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn list_owners(
    State(svc): State<Arc<OwnerService>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.list_owners(&auth).await?))
}

pub async fn update_owner(
    State(svc): State<Arc<OwnerService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateOwnerRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.update_owner(id, payload, &auth).await?))
}

pub async fn delete_owner(
    State(svc): State<Arc<OwnerService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    svc.delete_owner(id, &auth).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn resend_owner_invite(
    State(svc): State<Arc<OwnerService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<ResendOwnerInviteRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.resend_invite(payload, &auth).await?))
}
