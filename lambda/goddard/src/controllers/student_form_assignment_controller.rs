use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use std::sync::Arc;

use crate::error::AppError;
use crate::models::student_form_assignment::{
    CreateStudentFormAssignmentRequest, UpdateStudentFormAssignmentRequest,
    StudentFormAssignmentResponse, GetStudentFormAssignmentsQuery,
    DeleteStudentFormAssignmentQuery, DeleteStudentFormAssignmentResponse,
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