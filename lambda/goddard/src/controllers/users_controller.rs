use axum::{
    extract::{State, Path, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    User, UserRole, CreateUserRequest, QueryParams,
    ApiResponse, PaginatedResponse, PaginationMeta, PaginationLinks
};
use crate::db::users;

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
}

pub async fn list_users(
    Extension(user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<User>>, StatusCode> {
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

    // Get users from database
    let role_filter = params.role.clone();
    let users_list = users::get_users_by_school(&pool, school_id, params.role, page, limit)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch users: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get total count for pagination
    let total_count = users::count_users_by_school(&pool, school_id, role_filter)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count users: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: users_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/users?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/users?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/users?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/users?page=1&limit={}", limit),
            last: format!("/users?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn create_user(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
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

    // Create user in database
    let new_user = users::create_user(&pool, school_id, request, current_user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create user: {:?}", e);
            match e {
                crate::db::DbError::DuplicateRecord => StatusCode::CONFLICT,
                crate::db::DbError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                crate::db::DbError::Unauthorized => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(ApiResponse {
        data: new_user,
    }))
}

pub async fn get_user(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
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

    // Check permissions - user can view their own profile or admin can view all
    // For now, we'll allow access if it's the same user or if they're in the same school
    if current_user_id != user_id {
        // TODO: Check if current_user is admin or has permission to view this user
        // For now, just ensure they're in the same school by the Extension middleware
    }

    // Get user by ID from database
    let user = users::get_user_by_id(&pool, user_id, school_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user: {:?}", e);
            match e {
                crate::db::DbError::NotFound => StatusCode::NOT_FOUND,
                crate::db::DbError::Unauthorized => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(ApiResponse {
        data: user,
    }))
}

pub async fn update_user(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
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

    // Check permissions - user can update their own profile or admin can update all
    if current_user_id != user_id {
        // TODO: Check if current_user is admin or has permission to update this user
        // For now, just ensure they're in the same school by the Extension middleware
    }

    // Update user in database
    let updated_user = users::update_user(
        &pool,
        user_id,
        school_id,
        request.first_name,
        request.last_name,
        request.phone,
        current_user_id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update user: {:?}", e);
        match e {
            crate::db::DbError::NotFound => StatusCode::NOT_FOUND,
            crate::db::DbError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            crate::db::DbError::Unauthorized => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(ApiResponse {
        data: updated_user,
    }))
}

pub async fn delete_user(
    Extension(current_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Path(user_id): Path<Uuid>,
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

    // Check permissions - only admins can delete users
    // TODO: Implement proper role-based permission checking
    if current_user_id == user_id {
        // Users cannot delete themselves
        return Err(StatusCode::FORBIDDEN);
    }

    // Soft delete user by setting is_active to false
    let result = sqlx::query!(
        r#"
        UPDATE users
        SET
            is_active = false,
            updated_by = $3,
            updated_at = NOW()
        WHERE id = $1
            AND school_id = $2
            AND is_active = true
        "#,
        user_id,
        school_id,
        current_user_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete user: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}