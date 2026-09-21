use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json, Extension,
};
use std::sync::Arc;
use crate::middleware::auth::AuthContext;
use crate::models::requests::{CreateExpenseBody, ListExpensesParams};
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
