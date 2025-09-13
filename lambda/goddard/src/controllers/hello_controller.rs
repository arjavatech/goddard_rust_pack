use axum::{
    extract::{Path, Extension},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::{info, error};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{ApiResponse, HelloResponse, HealthResponse, ErrorResponse};

pub type HandlerResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

pub async fn hello_world(
    Extension(request_id): Extension<String>,
) -> HandlerResult<impl IntoResponse> {
    info!("Hello World endpoint called with request_id: {}", request_id);

    let response_data = HelloResponse {
        message: "Hello from Goddard Backend!".to_string(),
        name: None,
        request_id: request_id.clone(),
    };

    let api_response = ApiResponse::success(response_data, "Welcome to the Goddard Backend API!");

    Ok((StatusCode::OK, Json(api_response)))
}

pub async fn health_check(
    Extension(request_id): Extension<String>,
) -> HandlerResult<impl IntoResponse> {
    info!("Health check endpoint called with request_id: {}", request_id);

    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let response_data = HealthResponse {
        status: "healthy".to_string(),
        service: "goddard-backend-lambda".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime,
    };

    let api_response = ApiResponse::success(response_data, "Service is healthy");

    Ok((StatusCode::OK, Json(api_response)))
}

pub async fn hello_name(
    Path(name): Path<String>,
    Extension(request_id): Extension<String>,
) -> HandlerResult<impl IntoResponse> {
    info!("Hello name endpoint called with name: {}, request_id: {}", name, request_id);

    if name.trim().is_empty() {
        error!("Empty name provided");
        let error_response = ErrorResponse {
            error: "Bad Request".to_string(),
            message: "Name cannot be empty".to_string(),
            request_id,
            timestamp: chrono::Utc::now(),
        };
        return Err((StatusCode::BAD_REQUEST, Json(error_response)));
    }

    let response_data = HelloResponse {
        message: format!("Hello, {}! Welcome to the Goddard Backend", name),
        name: Some(name),
        request_id: request_id.clone(),
    };

    let api_response = ApiResponse::success(response_data, "Personalized greeting generated");

    Ok((StatusCode::OK, Json(api_response)))
}