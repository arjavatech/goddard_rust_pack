use std::sync::Arc;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::drive::*;
use crate::services::drive_service::DriveService;

pub async fn get_upload_url(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<DriveUploadUrlRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.get_upload_url(payload, &auth).await?))
}

pub async fn register_file(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<DriveRegisterFileRequest>,
) -> Result<impl IntoResponse, AppError> {
    let item = svc.register_file(payload, &auth).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

pub async fn create_folder(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<DriveCreateFolderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let item = svc.create_folder(payload, &auth).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

pub async fn list_items(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<DriveListQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.list_items(query, &auth).await?))
}

pub async fn get_download_url(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<DriveDownloadQuery>,
) -> Result<impl IntoResponse, AppError> {
    let download = query.download.unwrap_or(false);
    Ok(Json(svc.get_download_url(id, download, &auth).await?))
}

pub async fn rename_item(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(payload): Json<DriveRenameRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.rename_item(id, payload.name, &auth).await?))
}

pub async fn delete_item(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    svc.delete_item(id, &auth).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_item(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(payload): Json<DriveMoveRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.move_item(id, payload, &auth).await?))
}

pub async fn copy_item(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let item = svc.copy_item(id, &auth).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

pub async fn list_admins(
    State(svc): State<Arc<DriveService>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.list_admins(&auth).await?))
}
