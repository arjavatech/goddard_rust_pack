use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    Enrollment, EnrollmentWorkflowStatus, AdminApprovalStatus, EnrollmentProgress,
    CreateEnrollmentRequest, UpdateEnrollmentRequest, ApproveEnrollmentRequest, RejectEnrollmentRequest,
    QueryParams, ApiResponse, PaginatedResponse, PaginationMeta, PaginationLinks
};
// use crate::db::enrollments;

pub async fn list_enrollments(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<Enrollment>>, StatusCode> {
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
    let enrollments_list = vec![];
    let total_count = 0i64;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: enrollments_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/enrollments?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/enrollments?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/enrollments?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/enrollments?page=1&limit={}", limit),
            last: format!("/enrollments?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn create_enrollment(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Json(request): Json<CreateEnrollmentRequest>,
) -> Result<Json<ApiResponse<Enrollment>>, StatusCode> {
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

pub async fn get_enrollment(
    Extension(_current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(enrollment_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Enrollment>>, StatusCode> {
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

pub async fn update_enrollment(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(enrollment_id): Path<Uuid>,
    Json(request): Json<UpdateEnrollmentRequest>,
) -> Result<Json<ApiResponse<Enrollment>>, StatusCode> {
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

pub async fn approve_enrollment(
    Extension(_user_id): Extension<Uuid>,
    Extension(_school_id): Extension<Uuid>,
    Path(_enrollment_id): Path<Uuid>,
    Json(_request): Json<ApproveEnrollmentRequest>,
) -> Result<Json<ApiResponse<Enrollment>>, StatusCode> {
    // TODO: Implement enrollment approval logic
    Err(StatusCode::NOT_IMPLEMENTED)
}

pub async fn reject_enrollment(
    Extension(_user_id): Extension<Uuid>,
    Extension(_school_id): Extension<Uuid>,
    Path(_enrollment_id): Path<Uuid>,
    Json(_request): Json<RejectEnrollmentRequest>,
) -> Result<Json<ApiResponse<Enrollment>>, StatusCode> {
    // TODO: Implement enrollment rejection logic
    Err(StatusCode::NOT_IMPLEMENTED)
}