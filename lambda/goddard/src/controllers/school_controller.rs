use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::Deserialize;
use uuid::Uuid;
use crate::models::schema::{
    School, ApiResponse, ApiError, PaginatedResponse,
    PaginationMeta, PaginationLinks, QueryParams
};
use crate::db::schools;

#[derive(Deserialize)]
pub struct SchoolId {
    school_id: Uuid,
}

pub async fn get_school() -> Result<Json<ApiResponse<School>>, StatusCode> {
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

    // Get first school from database
    let school = schools::get_first_school(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch school: {:?}", e);
            match e {
                crate::db::DbError::NotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(ApiResponse {
        data: school,
    }))
}

pub async fn get_school_by_id(
    Path(school_id): Path<Uuid>,
) -> Result<Json<ApiResponse<School>>, StatusCode> {
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

    // Get school by ID from database
    let school = schools::get_school_by_id(&pool, school_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch school: {:?}", e);
            match e {
                crate::db::DbError::NotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(ApiResponse {
        data: school,
    }))
}

#[derive(Deserialize)]
pub struct CreateSchoolRequest {
    pub name: String,
    pub subdomain: String,
    pub settings: Option<std::collections::HashMap<String, serde_json::Value>>,
}

pub async fn get_all_schools(
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<School>>, StatusCode> {
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

    // Get schools from database
    let schools_list = schools::get_all_schools(&pool, page, limit)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch schools: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get total count for pagination
    let total_count = schools::count_all_schools(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count schools: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: schools_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/schools?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/schools?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/schools?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/schools?page=1&limit={}", limit),
            last: format!("/schools?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn create_school(
    Json(request): Json<CreateSchoolRequest>,
) -> Result<Json<ApiResponse<School>>, StatusCode> {
    // TODO: Check super admin permissions
    // TODO: Create default form templates for school

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

    // Create school in database
    let new_school = schools::create_school(&pool, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create school: {:?}", e);
            match e {
                crate::db::DbError::DuplicateRecord => StatusCode::CONFLICT,
                crate::db::DbError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(ApiResponse {
        data: new_school,
    }))
}
