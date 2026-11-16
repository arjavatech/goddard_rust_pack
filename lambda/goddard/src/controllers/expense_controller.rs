use axum::{
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;
use crate::middleware::auth::AuthContext;
use crate::models::requests::{CreateExpenseBody, ListExpensesParams, UpdateExpenseBody};
use crate::services::request_service::RequestService;
use crate::utils::response::ResponseUtils;
use crate::error::error_types::AppError;

pub async fn list_expenses(
    State(service): State<Arc<RequestService>>,
    Query(params): Query<ListExpensesParams>,
) -> Result<impl IntoResponse, AppError> {
    let result = service.list_expenses(params).await?;
    Ok(ResponseUtils::success(result))
}

pub async fn create_expense(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateExpenseBody>,
) -> Result<impl IntoResponse, AppError> {
    let result = service.create_manual_expense(&auth, body).await?;
    Ok(ResponseUtils::created(result))
}

pub async fn update_expense(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateExpenseBody>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ResponseUtils::success(service.update_expense(&auth, id, body).await?))
}

pub async fn delete_expense(
    State(service): State<Arc<RequestService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    service.delete_expense(&auth, id).await?;
    Ok(ResponseUtils::success(serde_json::json!({ "success": true })))
}
