use axum::{
    extract::{Extension, Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;
use chrono::NaiveDate;

use crate::{
    middleware::auth::{AuthContext, validate_parent_access},
    services::portal_service::PortalService,
    utils::ResponseUtils,
    error::AppError,
};

// =============================
// Request/Response Models
// =============================

#[derive(Debug, Serialize)]
pub struct UserContextResponse {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub parent_id: Option<Uuid>,
    pub school_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChildResponse {
    pub child_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub dob: NaiveDate,
    pub age: i32,
    pub class_name: String,
    pub enrollment_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentProgress {
    pub total_forms: i64,
    pub completed_forms: i64,
    pub completion_percentage: i32,
}

#[derive(Debug, Serialize)]
pub struct ChildProfileResponse {
    pub child_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub dob: NaiveDate,
    pub age: i32,
    pub class_name: String,
    pub enrollment_id: Option<Uuid>,
    pub enrollment_progress: EnrollmentProgress,
}

#[derive(Debug, Serialize)]
pub struct ChildFormResponse {
    pub assignment_id: Uuid,
    pub form_template_id: Uuid,
    pub title: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub launch_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClassroomDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub capacity: Option<i32>,
    pub age_group: Option<String>,
    pub teachers: Vec<String>,
    pub notes: Option<String>,
    pub enrolled_count: i64,
    pub available_spots: i32,
}

#[derive(Debug, Serialize)]
pub struct ClassroomFormResponse {
    pub form_template_id: Uuid,
    pub title: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub assigned_by: String,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AssignClassroomFormRequest {
    pub form_template_id: Uuid,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssignClassroomFormResponse {
    pub id: Uuid,
    pub classroom_id: Uuid,
    pub form_template_id: Uuid,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub assigned_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct PhoneNumbers {
    pub primary: Option<String>,
    pub secondary: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MailingAddress {
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LinkedChild {
    pub child_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub classroom: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParentProfileResponse {
    pub id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub phone_numbers: PhoneNumbers,
    pub mailing_address: MailingAddress,
    pub linked_children: Vec<LinkedChild>,
}

#[derive(Debug, Serialize)]
pub struct GuardianReference {
    pub parent_id: Uuid,
    pub relationship: Option<String>,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ChildDemographicsResponse {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub dob: NaiveDate,
    pub age: i32,
    pub classroom_id: Option<Uuid>,
    pub classroom_name: Option<String>,
    pub guardian_references: Vec<GuardianReference>,
}

// =============================
// Parent Portal APIs
// =============================

/// GET /users/me (Enhancement)
/// Get current user context with parent_id and school_id when authenticated as a parent
pub async fn get_user_context(
    Extension(auth): Extension<AuthContext>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    let response = portal_service.get_user_context(&auth).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /parents/{parent_id}/children
/// Get list of children linked to the logged-in parent
pub async fn get_parent_children(
    Extension(auth): Extension<AuthContext>,
    Path(parent_id): Path<Uuid>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify the parent can only access their own data or admin can access any
    validate_parent_access(&auth, &parent_id)?;

    let response = portal_service.get_parent_children(&parent_id).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /parents/{parent_id}/children/{child_id}/profile
/// Get detailed child metadata for dashboard cards
pub async fn get_child_profile(
    Extension(auth): Extension<AuthContext>,
    Path((parent_id, child_id)): Path<(Uuid, Uuid)>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify the parent can only access their own child's data or admin can access any
    validate_parent_access(&auth, &parent_id)?;

    let response = portal_service.get_child_profile(&parent_id, &child_id).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /parents/{parent_id}/children/{child_id}/forms
/// Get assigned forms with rich metadata for a specific child
pub async fn get_child_forms(
    Extension(auth): Extension<AuthContext>,
    Path((parent_id, child_id)): Path<(Uuid, Uuid)>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify the parent can only access their own child's data or admin can access any
    validate_parent_access(&auth, &parent_id)?;

    let response = portal_service.get_child_forms(&parent_id, &child_id).await?;
    Ok(ResponseUtils::success(response))
}

// =============================
// Admin Portal APIs
// =============================

/// GET /classrooms/{id}
/// Get detailed classroom profile (Admin/SuperAdmin only)
pub async fn get_classroom_details(
    Extension(auth): Extension<AuthContext>,
    Path(classroom_id): Path<Uuid>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify admin permissions
    portal_service.verify_admin_access(&auth)?;

    let response = portal_service.get_classroom_details(&classroom_id).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /classrooms/{id}/forms
/// List forms assigned to a classroom (Admin/SuperAdmin only)
pub async fn get_classroom_forms(
    Extension(auth): Extension<AuthContext>,
    Path(classroom_id): Path<Uuid>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify admin permissions
    portal_service.verify_admin_access(&auth)?;

    let response = portal_service.get_classroom_forms(&classroom_id).await?;
    Ok(ResponseUtils::success(response))
}

/// POST /classrooms/{id}/forms
/// Assign form to classroom (Admin/SuperAdmin only)
pub async fn assign_classroom_form(
    Extension(auth): Extension<AuthContext>,
    Path(classroom_id): Path<Uuid>,
    State(portal_service): State<Arc<PortalService>>,
    Json(payload): Json<AssignClassroomFormRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Verify admin permissions
    portal_service.verify_admin_access(&auth)?;

    let response = portal_service.assign_classroom_form(
        &classroom_id,
        payload,
        &auth.email
    ).await?;
    Ok(ResponseUtils::created(response))
}

/// DELETE /classrooms/{id}/forms/{form_id}
/// Remove form from classroom (Admin/SuperAdmin only)
pub async fn remove_classroom_form(
    Extension(auth): Extension<AuthContext>,
    Path((classroom_id, form_id)): Path<(Uuid, Uuid)>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify admin permissions
    portal_service.verify_admin_access(&auth)?;

    portal_service.remove_classroom_form(&classroom_id, &form_id).await?;
    Ok(ResponseUtils::success_with_message("Form assignment removed successfully"))
}

/// GET /parents/{parent_id}
/// Get parent profile (Parents can access their own, Admins can access any)
pub async fn get_parent_profile(
    Extension(auth): Extension<AuthContext>,
    Path(parent_id): Path<Uuid>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Allow parents to access their own profile OR admins to access any
    validate_parent_access(&auth, &parent_id)?;

    let response = portal_service.get_parent_profile(&parent_id).await?;
    Ok(ResponseUtils::success(response))
}

/// GET /children/{child_id}
/// Get child demographics (Admin/SuperAdmin only)
pub async fn get_child_demographics(
    Extension(auth): Extension<AuthContext>,
    Path(child_id): Path<Uuid>,
    State(portal_service): State<Arc<PortalService>>,
) -> Result<impl IntoResponse, AppError> {
    // Verify admin permissions
    portal_service.verify_admin_access(&auth)?;

    let response = portal_service.get_child_demographics(&child_id).await?;
    Ok(ResponseUtils::success(response))
}