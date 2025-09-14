use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    Classroom, AgeGroup, CreateClassroomRequest,
    QueryParams, ApiResponse, PaginatedResponse, PaginationMeta, PaginationLinks
};
// use crate::db::classrooms;

#[derive(Deserialize)]
pub struct UpdateClassroomRequest {
    pub name: Option<String>,
    pub age_group: Option<AgeGroup>,
    pub capacity: Option<i32>,
    pub teacher_id: Option<Uuid>,
    pub schedule: Option<serde_json::Value>,
}

pub async fn list_classrooms(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<Classroom>>, StatusCode> {
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
    let classrooms_list = vec![];
    let total_count = 0i64;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: classrooms_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/classrooms?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/classrooms?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/classrooms?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/classrooms?page=1&limit={}", limit),
            last: format!("/classrooms?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn create_classroom(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Json(request): Json<CreateClassroomRequest>,
) -> Result<Json<ApiResponse<Classroom>>, StatusCode> {
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