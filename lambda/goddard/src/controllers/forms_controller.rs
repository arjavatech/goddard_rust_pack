use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    FormTemplate, FormType, FormStatus, FormSubmission, SubmissionStatus,
    CreateFormTemplateRequest, FilloutWebhookRequest,
    QueryParams, ApiResponse, PaginatedResponse, PaginationMeta, PaginationLinks
};
// use crate::db::forms;

#[derive(Deserialize)]
pub struct UpdateFormTemplateRequest {
    pub name: Option<String>,
    pub form_type: Option<FormType>,
    pub config: Option<serde_json::Value>,
    pub is_required: Option<bool>,
}

pub async fn list_form_templates(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<FormTemplate>>, StatusCode> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);

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
    let forms_list = vec![];
    let total_count = 0i64;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: forms_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/forms/templates?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/forms/templates?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/forms/templates?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/forms/templates?page=1&limit={}", limit),
            last: format!("/forms/templates?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn create_form_template(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Json(request): Json<CreateFormTemplateRequest>,
) -> Result<Json<ApiResponse<FormTemplate>>, StatusCode> {
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

pub async fn handle_fillout_webhook(
    Json(_request): Json<FilloutWebhookRequest>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // TODO: Process fillout webhook data
    // TODO: Create form submission record
    // TODO: Update enrollment status if needed
    tracing::info!("Received fillout webhook");

    Ok(Json(ApiResponse {
        data: (),
    }))
}