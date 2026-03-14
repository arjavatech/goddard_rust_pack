use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Json, IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::{AuthContext, validate_parent_access};
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
    println!("[DEBUG] Starting student form assignment creation");

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

    println!("[DEBUG] Request data: {:?}", request);

    // Create student form assignment
    match service.create_student_form_assignment(request).await {
        Ok(assignment) => {
            println!("[DEBUG] Student form assignment created successfully");
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
    println!("[DEBUG] GET Assignments: Starting request for school: {}", query.school_id);

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
    println!("[DEBUG] GET Assignments: Authentication successful");

    let assignments = service
        .get_assignments_by_school(query.school_id)
        .await?;

    println!("[DEBUG] GET Assignments: Query completed successfully, found {} assignments", assignments.len());
    Ok(Json(assignments))
}

// Update Student Form Assignment (Protected - Admin/SuperAdmin)
pub async fn update_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<UpdateStudentFormAssignmentRequest>,
) -> Result<Json<StudentFormAssignmentResponse>, AppError> {
    println!("[DEBUG] PUT Assignment: Starting request for ID: {}", request.id);

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
    println!("[DEBUG] PUT Assignment: Authentication successful");

    let assignment = service
        .update_student_form_assignment(request)
        .await?;

    println!("[DEBUG] PUT Assignment: Update completed successfully");
    Ok(Json(assignment))
}

// Delete Student Form Assignment (Protected - Admin/SuperAdmin)
pub async fn delete_student_form_assignment(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Query(query): Query<DeleteStudentFormAssignmentQuery>,
) -> Result<Json<DeleteStudentFormAssignmentResponse>, AppError> {
    println!("[DEBUG] DELETE Assignment: Starting request for assignment: {}, school: {}",
             query.assignment_id, query.school_id);

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
    println!("[DEBUG] DELETE Assignment: Authentication successful");

    let response = service
        .delete_student_form_assignment(query.assignment_id, query.school_id)
        .await?;

    println!("[DEBUG] DELETE Assignment: Deletion completed successfully");
    Ok(Json(response))
}

// Bulk Assign Forms to Students (Protected - Admin/SuperAdmin)
pub async fn bulk_assign_forms_to_students(
    State(service): State<Arc<StudentFormAssignmentService>>,
    headers: HeaderMap,
    Json(request): Json<BulkAssignFormRequest>,
) -> Result<(StatusCode, Json<BulkAssignFormResponse>), AppError> {
    println!("[DEBUG] BULK ASSIGN: Starting bulk form assignment");
    println!("[DEBUG] BULK ASSIGN: School ID: {}, Number of assignments: {}",
             request.school_id, request.assignments.len());

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
    println!("[DEBUG] BULK ASSIGN: Authentication successful");

    // Perform bulk assignment
    match service.bulk_assign_forms(request).await {
        Ok(response) => {
            println!("[DEBUG] BULK ASSIGN: Successfully assigned {} forms, {} failed",
                     response.successful.len(), response.failed.len());
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
    println!("[DEBUG] ASSIGN TO SCHOOL: Starting assignment");
    println!("[DEBUG] ASSIGN TO SCHOOL: School ID: {}, Form Template ID: {}, Is Required: {:?}",
             request.school_id, request.form_template_id, request.is_required);

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
    println!("[DEBUG] ASSIGN TO SCHOOL: Authentication successful");

    // Perform assignment to all active students
    match service.assign_form_to_school_students(request).await {
        Ok(response) => {
            println!("[DEBUG] ASSIGN TO SCHOOL: Successfully assigned to {} students. Total: {}, Already assigned: {}, Newly assigned: {}",
                     response.newly_assigned, response.total_active_students,
                     response.students_already_assigned, response.newly_assigned);
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            println!("[ERROR] ASSIGN TO SCHOOL: Failed with error: {:?}", e);
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
    println!("[DEBUG] DOWNLOAD ZIP: Starting for enrollment: {}", enrollment_id);
    println!("[DEBUG] DOWNLOAD ZIP: Auth context - User: {}, Role: {:?}", auth.user_id, auth.role);

    // Lookup parent_id for this enrollment and validate access
    let parent_id = service.get_enrollment_parent_id(enrollment_id).await?;
    validate_parent_access(&auth, &parent_id)?;
    println!("[DEBUG] DOWNLOAD ZIP: Access validation passed");

    // Download and create ZIP
    let (zip_bytes, filename) = service.download_enrollment_forms_zip(enrollment_id).await?;

    println!("[DEBUG] DOWNLOAD ZIP: Returning ZIP file: {} ({} bytes)", filename, zip_bytes.len());

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
        ],
        zip_bytes,
    ).into_response())
}