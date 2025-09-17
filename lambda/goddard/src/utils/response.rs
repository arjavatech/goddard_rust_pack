use serde::Serialize;
use axum::{http::StatusCode, response::IntoResponse, Json};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

pub struct ResponseUtils;

impl ResponseUtils {
    pub fn success<T: Serialize>(data: T) -> impl IntoResponse {
        let response = ApiResponse {
            success: true,
            data: Some(data),
            message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        (StatusCode::OK, Json(response))
    }

    pub fn created<T: Serialize>(data: T) -> impl IntoResponse {
        let response = ApiResponse {
            success: true,
            data: Some(data),
            message: Some("Resource created successfully".to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        (StatusCode::CREATED, Json(response))
    }

    pub fn no_content() -> impl IntoResponse {
        StatusCode::NO_CONTENT
    }

    pub fn paginated<T: Serialize>(
        data: Vec<T>,
        page: u32,
        per_page: u32,
        total: u64,
    ) -> impl IntoResponse {
        let total_pages = (total as f64 / per_page as f64).ceil() as u32;

        let response = PaginatedResponse {
            data,
            pagination: PaginationInfo {
                page,
                per_page,
                total,
                total_pages,
            },
        };

        (StatusCode::OK, Json(response))
    }
}