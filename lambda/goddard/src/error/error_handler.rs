use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use super::error_types::AppError;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.to_status_code();
        let error_response = self.to_error_response();

        // Log error for monitoring
        log_error(&self, status);

        (status, Json(error_response)).into_response()
    }
}

fn log_error(error: &AppError, status: StatusCode) {
    match status {
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::BAD_GATEWAY => {
            tracing::error!("Error: {} - Status: {}", error, status);
        }
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            tracing::warn!("Client error: {} - Status: {}", error, status);
        }
        _ => {
            tracing::info!("Request error: {} - Status: {}", error, status);
        }
    }
}

// Result type alias for convenience
pub type ApiResult<T> = Result<T, AppError>;