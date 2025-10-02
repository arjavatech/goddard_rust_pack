use axum::{
    extract::{State, Extension, Path},
    response::IntoResponse,
    Json,
    http::StatusCode,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthContext,
    services::enrollment_service::EnrollmentService,
    models::enrollment::{ParentInviteRequest, ResendConfirmationRequest, AddChildRequest, GetParentDetailsBySchoolRequest, GetEnrollmentChildrenRequest, GetClassWiseCountRequest, GetSchoolFormsRequest},
    utils::ResponseUtils,
    error::AppError,
};

/// POST /enrollments/parent-invite
/// Create a parent invite for child enrollment (JWT protected - Admin/SuperAdmin)
pub async fn create_parent_invite(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Json(payload): Json<ParentInviteRequest>,
) -> Result<impl IntoResponse, AppError> {
    // JWT admin_only middleware will handle authentication and authorization
    // The service will validate business logic and permissions
    println!("[DEBUG] Creating parent invite - User: {}, Role: {:?}", auth.email, auth.role);
    let response = enrollment_service.create_parent_invite(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// POST /enrollments/resend-confirmation
/// Resend parent confirmation email using Supabase auth ID (API Key protected)
pub async fn resend_parent_confirmation(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Json(payload): Json<ResendConfirmationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // API key middleware will handle authentication
    // parent_id is the Supabase auth user ID, not local users table ID
    let response = enrollment_service.resend_parent_confirmation(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// POST /enrollments/add-child
/// Add additional child to existing parent (JWT protected - Admin/SuperAdmin)
pub async fn add_child(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Json(payload): Json<AddChildRequest>,
) -> Result<impl IntoResponse, AppError> {
    // JWT admin_only middleware will handle authentication and authorization
    // The service will validate business logic and permissions
    println!("[DEBUG] Adding child - User: {}, Role: {:?}", auth.email, auth.role);
    let response = enrollment_service.add_child(payload).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /enrollments/parent-details-by-school?school_id={uuid}
/// Get parent details by school including auth information (API Key protected)
pub async fn get_parent_details_by_school(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    axum::extract::Query(query): axum::extract::Query<GetParentDetailsBySchoolRequest>,
) -> Result<impl IntoResponse, AppError> {
    // API key middleware will handle authentication
    let response = enrollment_service.get_parent_details_by_school(query).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /enrollments/children-forms?school_id={uuid}
/// Get enrollment children with form assignments (JWT protected)
pub async fn get_enrollment_children_with_forms(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    axum::extract::Query(query): axum::extract::Query<GetEnrollmentChildrenRequest>,
) -> Result<impl IntoResponse, AppError> {
    // JWT admin_only middleware will handle authentication and authorization
    println!("[DEBUG] Getting children forms - User: {}, Role: {:?}", auth.email, auth.role);
    let response = enrollment_service.get_enrollment_children_with_forms(query).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /enrollments?school_id={uuid}
/// Get all enrollment form details by school (API Key protected)
pub async fn get_school_forms(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    axum::extract::Query(query): axum::extract::Query<GetSchoolFormsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = enrollment_service.get_school_forms(query).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /enrollments/class-wise-count?school_id={uuid}
/// Get class-wise child count details (API Key protected)
pub async fn get_class_wise_count(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    axum::extract::Query(query): axum::extract::Query<GetClassWiseCountRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = enrollment_service.get_class_wise_count(query).await?;
    Ok(ResponseUtils::success(response))
}

/// DELETE /parent/:parent_id
/// Deactivate parent and all related children and enrollments (JWT protected - Admin/SuperAdmin)
pub async fn deactivate_parent(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(parent_id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    println!("[DEBUG] Deactivating parent - User: {}, Role: {:?}, Parent ID: {}",
        auth.email, auth.role, parent_id);

    let response = enrollment_service.deactivate_parent(parent_id).await?;
    Ok((StatusCode::OK, ResponseUtils::success(response)))
}