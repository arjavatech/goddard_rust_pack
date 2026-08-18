use axum::{
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;
use crate::middleware::auth::AuthContext;
use crate::models::requests::{CreateRequestBody, UpdateRequestStatusBody, UpdateExpectedCompletionDateBody, UpdateRequestBody, PayRequestBody, ListRequestsParams};
use crate::services::request_service::RequestService;
use crate::utils::response::ResponseUtils;
use crate::error::error_types::AppError;

pub async fn list_requests(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<ListRequestsParams>,
) -> Result<impl IntoResponse, AppError> {
    let result = service.list_requests(&auth, params).await?;
    Ok(ResponseUtils::success(result))
}

pub async fn create_request(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateRequestBody>,
) -> Result<impl IntoResponse, AppError> {
    let result = service.create_request(&auth, body).await?;
    Ok(ResponseUtils::created(result))
}

pub async fn update_request_status(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRequestStatusBody>,
) -> Result<impl IntoResponse, AppError> {
    let result = service.update_request_status(&auth, id, body).await?;
    Ok(ResponseUtils::success(result))
}

pub async fn update_expected_completion_date(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateExpectedCompletionDateBody>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ResponseUtils::success(service.update_expected_completion_date(&auth, id, body).await?))
}

pub async fn update_request(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRequestBody>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ResponseUtils::success(service.update_request(&auth, id, body).await?))
}

pub async fn pay_request(
    State(service): State<Arc<RequestService>>,
    Path(id): Path<Uuid>,
    Json(body): Json<PayRequestBody>,
) -> Result<impl IntoResponse, AppError> {
    let result = service.pay_request(id, body).await?;
    Ok(ResponseUtils::success(result))
}

pub async fn delete_request(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    service.delete_request(&auth, id).await?;
    Ok(ResponseUtils::success(serde_json::json!({ "success": true })))
}
