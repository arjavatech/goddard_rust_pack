use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::error_types::AppError;
use crate::models::classroom::{CreateClassroomRequest, UpdateClassroomRequest, ClassroomResponse, ClassroomListResponse};
use crate::services::classroom_service::ClassroomService;

#[derive(Deserialize)]
pub struct ClassroomQueryParams {
    pub school_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct DeleteClassroomParams {
    pub classroom_id: Uuid,
    pub school_id: Uuid,
}

#[derive(serde::Serialize)]
pub struct DeleteResponse {
    pub message: String,
    pub classroom_id: Uuid,
    pub school_id: Uuid,
}

pub async fn create_classroom(
    State(classroom_service): State<Arc<ClassroomService>>,
    Json(payload): Json<CreateClassroomRequest>,
) -> Result<impl IntoResponse, AppError> {
    let classroom = classroom_service.create_classroom(payload).await?;
    let response = ClassroomResponse::from(classroom);

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_classrooms_by_school(
    State(classroom_service): State<Arc<ClassroomService>>,
    Query(params): Query<ClassroomQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id
        .ok_or_else(|| AppError::Validation("Missing required parameter: school_id".to_string()))?;

    tracing::info!("Fetching classrooms for school_id: {}", school_id);

    match classroom_service.get_classrooms_by_school(school_id).await {
        Ok(classrooms) => {
            let response: Vec<ClassroomListResponse> = classrooms
                .into_iter()
                .map(ClassroomListResponse::from)
                .collect();

            tracing::info!("Successfully fetched {} classrooms", response.len());
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            tracing::error!("Database error: {:?}", e);
            Err(AppError::Database(format!("Failed to fetch classrooms for school {}: {}", school_id, e)))
        }
    }
}

pub async fn update_classroom(
    State(classroom_service): State<Arc<ClassroomService>>,
    Json(payload): Json<UpdateClassroomRequest>,
) -> Result<impl IntoResponse, AppError> {
    let classroom = classroom_service.update_classroom(payload).await?;
    let response = ClassroomResponse {
        id: classroom.id,
        school_id: classroom.school_id,
        name: classroom.name,
        age_group: classroom.age_group,
        capacity: classroom.capacity,
        enrolled_count: classroom.enrolled_count,
        created_at: classroom.created_at,
        updated_at: classroom.updated_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_classroom(
    State(classroom_service): State<Arc<ClassroomService>>,
    Query(params): Query<DeleteClassroomParams>,
) -> Result<impl IntoResponse, AppError> {
    classroom_service.delete_classroom(params.classroom_id, params.school_id).await?;

    let response = DeleteResponse {
        message: "Classroom successfully deleted".to_string(),
        classroom_id: params.classroom_id,
        school_id: params.school_id,
    };

    Ok((StatusCode::OK, Json(response)))
}