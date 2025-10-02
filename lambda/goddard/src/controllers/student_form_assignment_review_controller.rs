use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Extension,
};
use std::sync::Arc;

use crate::error::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::student_form_assignment_review::{
    ReviewStudentFormAssignmentRequest, ReviewStudentFormAssignmentResponse,
};
use crate::services::StudentFormAssignmentService;

// Review Student Form Assignment (Protected - Admin/SuperAdmin)
// Updates the status of an assignment to approved or rejected with notes
pub async fn review_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Json(review_request): Json<ReviewStudentFormAssignmentRequest>,
) -> Result<(StatusCode, Json<ReviewStudentFormAssignmentResponse>), AppError> {
    println!("[DEBUG] Starting student form assignment review for ID: {}", review_request.assignment_id);

    // Auth context is already provided by middleware (jwt_or_api_key_admin_only)
    println!("[DEBUG] Authenticated user: {}, Role: {:?}, School: {}",
        auth.email, auth.role, auth.school_id);

    println!("[DEBUG] Review request - Status: {:?}, Notes: {:?}", review_request.status, review_request.notes);

    // Review the student form assignment
    match service.review_student_form_assignment(review_request).await {
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