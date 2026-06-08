use serde::Serialize;
use axum::http::StatusCode;

#[derive(Debug, thiserror::Error, Serialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("{0}")]
    Validation(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: String,
    pub timestamp: String,
}

impl AppError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Database(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_string(),
            message: self.user_message(),
            code: self.error_code(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn user_message(&self) -> String {
        match self {
            AppError::Validation(msg) => msg.clone(),
            AppError::Authentication(_) => "Authentication failed".to_string(),
            AppError::Authorization(_) => "Access denied".to_string(),
            AppError::NotFound(resource) => format!("{} not found", resource),
            AppError::Conflict(msg) => msg.clone(),
            _ => "An internal error occurred".to_string(),
        }
    }

    fn error_code(&self) -> String {
        match self {
            AppError::Database(_) => "DATABASE_ERROR".to_string(),
            AppError::Validation(_) => "VALIDATION_ERROR".to_string(),
            AppError::Authentication(_) => "AUTH_ERROR".to_string(),
            AppError::Authorization(_) => "AUTHORIZATION_ERROR".to_string(),
            AppError::NotFound(_) => "NOT_FOUND".to_string(),
            AppError::Conflict(_) => "CONFLICT".to_string(),
            AppError::ExternalService(_) => "EXTERNAL_SERVICE_ERROR".to_string(),
            AppError::Internal(_) => "INTERNAL_ERROR".to_string(),
        }
    }
}