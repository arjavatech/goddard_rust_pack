use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    EmailNotification, EmailType, CreateEmailNotificationRequest,
    QueryParams, ApiResponse
};
// use crate::db::notifications;

#[derive(Deserialize)]
pub struct UpdateNotificationRequest {
    pub email_type: Option<EmailType>,
    pub subject: Option<String>,
    pub template_data: Option<serde_json::Value>,
}

pub async fn list_additional_emails(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(_params): Query<QueryParams>,
) -> Result<Json<ApiResponse<Vec<EmailNotification>>>, StatusCode> {
    // Create database connection
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost/goddard_db".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // TODO: Re-enable when database module is fixed
    let notifications = vec![];

    Ok(Json(ApiResponse {
        data: notifications,
    }))
}

pub async fn create_additional_email(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Json(request): Json<CreateEmailNotificationRequest>,
) -> Result<Json<ApiResponse<EmailNotification>>, StatusCode> {
    // Create database connection
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost/goddard_db".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // TODO: Re-enable when database module is fixed
    Err(StatusCode::NOT_IMPLEMENTED)
}