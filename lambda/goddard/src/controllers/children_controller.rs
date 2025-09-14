use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    Child, AgeGroup, EnrollmentStatus, CreateChildRequest,
    QueryParams, ApiResponse, PaginatedResponse, PaginationMeta, PaginationLinks
};
// use crate::db::children;

#[derive(Deserialize)]
pub struct UpdateChildRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birth_date: Option<chrono::NaiveDate>,
    pub medical_info: Option<serde_json::Value>,
}

pub async fn list_children(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<Child>>, StatusCode> {
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
    // Get children from database
    let children_list = vec![];
    let total_count = 0i64;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: children_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/children?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/children?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/children?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/children?page=1&limit={}", limit),
            last: format!("/children?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn create_child(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Json(request): Json<CreateChildRequest>,
) -> Result<Json<ApiResponse<Child>>, StatusCode> {
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
    // Create child in database
    Err(StatusCode::NOT_IMPLEMENTED)
}

pub async fn get_child(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(child_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Child>>, StatusCode> {
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
    // Get child by ID from database
    // TODO: Check if current user is parent of this child or admin
    Err(StatusCode::NOT_IMPLEMENTED)
}

pub async fn update_child(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(child_id): Path<Uuid>,
    Json(request): Json<UpdateChildRequest>,
) -> Result<Json<ApiResponse<Child>>, StatusCode> {
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
    // Update child in database
    // let updated_child = children::update_child(
    //     &pool,
    //     child_id,
    //     school_id,
    //     request.first_name,
    //     request.last_name,
    //     request.birth_date,
    //     request.medical_info,
    //     current_user_id,
    //     .await
    //     .map_err(|e| {
    //         tracing::error!("Failed to update child: {:?}", e);
    //         match e {
    //             crate::db::DbError::NotFound => StatusCode::NOT_FOUND,
    //             crate::db::DbError::InvalidInput(_) => StatusCode::BAD_REQUEST,
    //             crate::db::DbError::Unauthorized => StatusCode::FORBIDDEN,
    //             _ => StatusCode::INTERNAL_SERVER_ERROR,
    //         }
    //     })?;

    Err(StatusCode::NOT_IMPLEMENTED)
}

pub async fn delete_child(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(child_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
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

    // Soft delete child by setting is_active to false
    let result = sqlx::query!(
        r#"
        UPDATE children
        SET
            is_active = false,
            updated_by = $3,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        child_id,
        school_id,
        current_user_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete child: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}