use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Extension,
};
use std::sync::Arc;

use crate::error::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::student_form_assignment_revoke::{
    RevokeStudentFormAssignmentRequest, RevokeStudentFormAssignmentResponse,
};
use crate::services::StudentFormAssignmentService;

// Revoke Student Form Assignment Approval (Protected - Admin/SuperAdmin)
// Reverts an approved assignment back to in_progress with audit notes
pub async fn revoke_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Json(revoke_request): Json<RevokeStudentFormAssignmentRequest>,
) -> Result<(StatusCode, Json<RevokeStudentFormAssignmentResponse>), AppError> {
    // Validate notes field is not empty
    if revoke_request.notes.trim().is_empty() {
        return Err(AppError::Validation(
            "Revocation reason (notes) is required".to_string(),
        ));
    }

    // Auth context is already provided by middleware (jwt_or_api_key_admin_only)

    // Revoke the student form assignment approval
    match service.revoke_student_form_assignment(revoke_request).await {
        Ok(assignment) => {
            Ok((StatusCode::OK, Json(assignment)))
        }
        Err(e) => {
            println!("[ERROR] Failed to revoke student form assignment: {:?}", e);
            Err(e)
        }
    }
}
