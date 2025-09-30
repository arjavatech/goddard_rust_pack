use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
};
use std::sync::Arc;

use crate::error::AppError;
use crate::models::student_form_assignment_review::{
    ReviewStudentFormAssignmentRequest, ReviewStudentFormAssignmentResponse,
};
use crate::services::StudentFormAssignmentService;

// Review Student Form Assignment (Protected - Admin/SuperAdmin)
// Updates the status of an assignment to approved or rejected with notes
pub async fn review_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<ReviewStudentFormAssignmentRequest>,
) -> Result<(StatusCode, Json<ReviewStudentFormAssignmentResponse>), AppError> {
    println!("[DEBUG] Starting student form assignment review for ID: {}", request.assignment_id);

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    println!("[DEBUG] API key extracted successfully");

    // Validate API key
    match service.validate_api_key(api_key).await {
        Ok(_) => println!("[DEBUG] API key validation passed"),
        Err(e) => {
            println!("[ERROR] API key validation failed: {:?}", e);
            return Err(e);
        }
    }

    println!("[DEBUG] Review request - Status: {:?}, Notes: {:?}", request.status, request.notes);

    // Review the student form assignment
    match service.review_student_form_assignment(request).await {
        Ok(assignment) => {
            println!("[DEBUG] Student form assignment reviewed successfully");
            Ok((StatusCode::OK, Json(assignment)))
        }
        Err(e) => {
            println!("[ERROR] Failed to review student form assignment: {:?}", e);
            Err(e)
        }
    }
}