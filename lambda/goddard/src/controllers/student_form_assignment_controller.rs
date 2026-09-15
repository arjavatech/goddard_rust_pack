use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Json, IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::{AuthContext, validate_parent_access, check_permission_admin_or_superadmin};
use crate::models::form_review_queue::FormReviewQueueQuery;
use crate::models::student_form_assignment::{
    CreateStudentFormAssignmentRequest, UpdateStudentFormAssignmentRequest,
    StudentFormAssignmentResponse, GetStudentFormAssignmentsQuery,
    DeleteStudentFormAssignmentQuery, DeleteStudentFormAssignmentResponse,
    BulkAssignFormRequest, BulkAssignFormResponse,
};
use crate::services::StudentFormAssignmentService;

// Create Student Form Assignment (Protected - Admin/SuperAdmin)
pub async fn create_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<CreateStudentFormAssignmentRequest>,
) -> Result<(StatusCode, Json<StudentFormAssignmentResponse>), AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;


    // Validate API key
    service.validate_api_key(api_key).await?;


    // Create student form assignment
    match service.create_student_form_assignment(request).await {
        Ok(assignment) => {
            Ok((StatusCode::CREATED, Json(assignment)))
        }
        Err(e) => {
            println!("[ERROR] Failed to create student form assignment: {:?}", e);
            Err(e)
        }
    }
}

// Get All Student Form Assignments by School (Protected - School Context)
pub async fn get_assignments_by_school(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Query(query): Query<GetStudentFormAssignmentsQuery>,
) -> Result<Json<Vec<StudentFormAssignmentResponse>>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] GET Assignments: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_api_key(api_key).await?;

    let assignments = service
        .get_assignments_by_school(query.school_id)
        .await?;

    Ok(Json(assignments))
}

pub async fn get_student_form_review_queue(
    State(service): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<FormReviewQueueQuery>,
) -> Result<Json<Vec<crate::models::form_review_queue::StudentFormReviewQueueItem>>, AppError> {
    check_permission_admin_or_superadmin(&auth, &query.school_id)?;
    Ok(Json(service.get_review_queue(&query).await?))
}

// Update Student Form Assignment (Protected - Admin/SuperAdmin)
pub async fn update_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<UpdateStudentFormAssignmentRequest>,
) -> Result<Json<StudentFormAssignmentResponse>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] PUT Assignment: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_api_key(api_key).await?;

    let assignment = service
        .update_student_form_assignment(request)
        .await?;

    Ok(Json(assignment))
}

// Delete Student Form Assignment (Protected - Admin/SuperAdmin)
pub async fn delete_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Query(query): Query<DeleteStudentFormAssignmentQuery>,
) -> Result<Json<DeleteStudentFormAssignmentResponse>, AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] DELETE Assignment: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_api_key(api_key).await?;

    let response = service
        .delete_student_form_assignment(query.assignment_id, query.school_id)
        .await?;

    Ok(Json(response))
}

// Bulk Assign Forms to Students (Protected - Admin/SuperAdmin)
pub async fn bulk_assign_forms_to_students(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<BulkAssignFormRequest>,
) -> Result<(StatusCode, Json<BulkAssignFormResponse>), AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] BULK ASSIGN: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_api_key(api_key).await?;

    // Perform bulk assignment
    match service.bulk_assign_forms(request).await {
        Ok(response) => {
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            println!("[ERROR] BULK ASSIGN: Failed with error: {:?}", e);
            Err(e)
        }
    }
}

// Assign form to all active students in a school (Protected - Admin/SuperAdmin)
pub async fn assign_form_to_school_students(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<crate::models::student_form_assignment::AssignFormToSchoolStudentsRequest>,
) -> Result<(StatusCode, Json<crate::models::student_form_assignment::AssignFormToSchoolStudentsResponse>), AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] ASSIGN TO SCHOOL: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_api_key(api_key).await?;

    // Perform assignment to all active students
    match service.assign_form_to_school_students(request).await {
        Ok(response) => {
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            println!("[ERROR] ASSIGN TO SCHOOL: Failed with error: {:?}", e);
            Err(e)
        }
    }
}

// Assign form to all active students in a class (Protected - Admin/SuperAdmin)
pub async fn assign_form_to_class_students(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<crate::models::student_form_assignment::AssignFormToClassStudentsRequest>,
) -> Result<(StatusCode, Json<crate::models::student_form_assignment::AssignFormToClassStudentsResponse>), AppError> {

    // Extract API key from X-API-Key header
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("[ERROR] ASSIGN TO CLASS: Missing X-API-Key header");
            AppError::Authentication("Missing X-API-Key header".to_string())
        })?;

    // Validate API key
    service.validate_api_key(api_key).await?;

    // Perform assignment to all active students in the class
    match service.assign_form_to_class_students(request).await {
        Ok(response) => {
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            println!("[ERROR] ASSIGN TO CLASS: Failed with error: {:?}", e);
            Err(e)
        }
    }
}

// Download ZIP of completed enrollment form PDFs
pub async fn download_enrollment_forms_zip(
    Extension(auth): Extension<AuthContext>,
    State(service): State<Arc<StudentFormAssignmentService>>,
    Path(enrollment_id): Path<Uuid>,
) -> Result<Response, AppError> {

    // Lookup parent_id and child name for this enrollment and validate access
    let (parent_id, child_first_name, child_last_name) = service.get_enrollment_parent_id(enrollment_id).await?;
    validate_parent_access(&auth, &parent_id)?;

    // Download and create ZIP
    let (zip_bytes, filename) = service.download_enrollment_forms_zip(enrollment_id, &child_first_name, &child_last_name).await?;


    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
        ],
        zip_bytes,
    ).into_response())
}
