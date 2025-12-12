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
    models::enrollment::{ParentInviteRequest, ResendConfirmationRequest, AddChildRequest, GetParentDetailsBySchoolRequest, GetEnrollmentChildrenRequest, GetClassWiseCountRequest, GetSchoolFormsRequest, GetClassBasedEnrollmentsRequest, UpdateChildStatusRequest},
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

/// GET /class-based-enrollments?school_id={uuid}&class_id={uuid}
/// Get class-based enrollment form details (API Key protected)
pub async fn get_class_based_enrollments(
    State(enrollment_service): State<Arc<EnrollmentService>>,
    axum::extract::Query(query): axum::extract::Query<GetClassBasedEnrollmentsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = enrollment_service.get_class_based_enrollments(query).await?;
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

/// PATCH /parent/:parent_id/activate
/// Activate parent and all related children and enrollments (JWT protected - Admin/SuperAdmin)
pub async fn activate_parent(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(parent_id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    println!("[DEBUG] Activating parent - User: {}, Role: {:?}, Parent ID: {}",
        auth.email, auth.role, parent_id);

    let response = enrollment_service.activate_parent(parent_id).await?;
    Ok((StatusCode::OK, ResponseUtils::success(response)))
}

/// PATCH /children/:child_id/status
/// Update child status to any value (JWT protected - Admin/SuperAdmin only)
pub async fn update_child_status(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(child_id): Path<Uuid>,
    Json(payload): Json<UpdateChildStatusRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!("[DEBUG] Updating child status - User: {}, Role: {:?}, Child ID: {}, Status: {}",
        auth.email, auth.role, child_id, payload.status);

    let response = enrollment_service.update_child_status(child_id, payload).await?;
    Ok(ResponseUtils::success(response))
}

// ==========================================
// CLASS TRANSITIONS CONTROLLER HANDLERS
// ==========================================

/// POST /enrollments/:enrollment_id/promote
/// Promote student to next class (creates new transition record) (JWT protected - Admin/SuperAdmin)
pub async fn promote_enrollment(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(enrollment_id): Path<Uuid>,
    Json(payload): Json<crate::models::enrollment::PromoteEnrollmentRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!("[DEBUG] Promoting enrollment - User: {}, Role: {:?}, Enrollment ID: {}, Target Classroom: {}",
        auth.email, auth.role, enrollment_id, payload.to_classroom_id);

    let response = enrollment_service.promote_enrollment(
        enrollment_id,
        payload,
        auth.user_id,
        auth.school_id
    ).await?;

    Ok(ResponseUtils::success(response))
}

/// PATCH /class-transitions/:transition_id
/// Edit existing transition record (no new entry created) (JWT protected - Admin/SuperAdmin)
pub async fn edit_class_transition(
    Extension(auth): Extension<AuthContext>,
    State(enrollment_service): State<Arc<EnrollmentService>>,
    Path(transition_id): Path<Uuid>,
    Json(payload): Json<crate::models::enrollment::EditClassTransitionRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!("[DEBUG] Editing class transition - User: {}, Role: {:?}, Transition ID: {}",
        auth.email, auth.role, transition_id);

    let response = enrollment_service.edit_class_transition(
        transition_id,
        payload,
        auth.school_id
    ).await?;

    Ok(ResponseUtils::success(response))
}