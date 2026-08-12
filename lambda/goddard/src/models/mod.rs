pub mod schema;
pub mod school;
pub mod classroom;
pub mod form_template;
pub mod class_form_override;
pub mod enrollment;
pub mod form_submission;
pub mod student_form_assignment;
pub mod student_form_assignment_review;
pub mod parent_details;
pub mod fillout;
pub mod admin;
pub mod email;
pub mod notification;
pub mod websocket_message;
pub mod employee;
pub mod requests;
pub mod upload;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: message.into(),
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: message.into(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HelloResponse {
    pub message: String,
    pub name: Option<String>,
    pub request_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub uptime: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}