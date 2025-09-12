use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ApiError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Internal server error: {message}")]
    InternalServerError { message: String },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },
}

impl ApiError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::BadRequest { .. } => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound { .. } => 404,
            Self::Conflict { .. } => 409,
            Self::InternalServerError { .. } => 500,
            Self::ServiceUnavailable { .. } => 503,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            Self::BadRequest { .. } => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::InternalServerError { .. } => "INTERNAL_SERVER_ERROR",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
        }
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_type: String,
    pub message: String,
    pub timestamp: String,
    pub request_id: Option<String>,
}

impl ErrorResponse {
    pub fn from_api_error(error: ApiError, request_id: Option<String>) -> Self {
        Self {
            error: error.to_string(),
            error_type: error.error_type().to_string(),
            message: error.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id,
        }
    }
}