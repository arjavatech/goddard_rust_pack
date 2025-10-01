use axum::{
    extract::{Extension, Path, State},
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    middleware::auth::{AuthContext, validate_parent_access},
    services::enrollment_service::EnrollmentService,
    utils::ResponseUtils,
    error::AppError,
};

/// GET /parent/{parent_id}
/// Get comprehensive details for a specific parent (JWT protected with parent isolation)
/// - Parents can only access their own data
/// - Admins/SuperAdmins can access any parent's data
pub async fn get_parent_details_by_id(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(parent_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    println!("[DEBUG] GET Parent Details: Starting request for parent_id: {}", parent_id);
    println!("[DEBUG] Auth context - User: {}, Role: {:?}", auth.user_id, auth.role);

    // CRITICAL SECURITY: Validate parent access
    // Parents can only access their own data
    // Admins/SuperAdmins can access any parent's data
    validate_parent_access(&auth, &parent_id)?;

    println!("[DEBUG] GET Parent Details: Access validation passed");

    // Get parent details from service
    let response = enrollment_service.get_parent_details_by_id(parent_id).await?;

    println!("[DEBUG] GET Parent Details: Successfully retrieved parent details");
    Ok(ResponseUtils::success(response))
}