use std::sync::Arc;
use axum::{extract::{Extension, Path, Query, State}, response::Json};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use futures::stream::StreamExt;

use crate::{
    error::AppError,
    middleware::auth::{AuthContext, check_permission_school_access},
    models::student_form_assignment::{
        ManualPdfUploadIntentRequest, ManualPdfUploadIntentResponse,
        ManualPdfCompleteUploadRequest, StudentFormAssignmentResponse,
    },
    services::StudentFormAssignmentService,
};

#[derive(Deserialize)]
pub struct SchoolQuery {
    pub school_id: Uuid,
}

#[derive(Serialize)]
pub struct FileAccessResponse {
    pub url: String,
}

pub async fn student_manual_pdf_upload_intent(
    State(svc): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Path(_id): Path<Uuid>,
    Json(body): Json<ManualPdfUploadIntentRequest>,
) -> Result<Json<ManualPdfUploadIntentResponse>, AppError> {
    check_permission_school_access(&auth, &body.school_id)?;
    Ok(Json(svc.create_manual_pdf_upload_intent(body).await?))
}

pub async fn student_manual_pdf_complete_upload(
    State(svc): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<ManualPdfCompleteUploadRequest>,
) -> Result<Json<StudentFormAssignmentResponse>, AppError> {
    let school_id = Uuid::parse_str(&body.storage_key.split('/').nth(3).unwrap_or(""))
        .map_err(|_| AppError::Validation("Invalid storage key format".to_string()))?;
    check_permission_school_access(&auth, &school_id)?;
    Ok(Json(svc.complete_manual_pdf_upload(id, school_id, body).await?))
}

pub async fn get_student_manual_pdf_url(
    State(svc): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(q): Query<SchoolQuery>,
) -> Result<Json<FileAccessResponse>, AppError> {
    check_permission_school_access(&auth, &q.school_id)?;
    let url = svc.get_manual_pdf_access_url(id, q.school_id).await?;
    Ok(Json(FileAccessResponse { url }))
}

pub async fn delete_student_manual_pdf(
    State(svc): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(q): Query<SchoolQuery>,
) -> Result<Json<StudentFormAssignmentResponse>, AppError> {
    check_permission_school_access(&auth, &q.school_id)?;
    Ok(Json(svc.remove_manual_pdf(id, q.school_id).await?))
}

pub async fn upload_student_manual_pdf(
    State(svc): State<Arc<StudentFormAssignmentService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(q): Query<SchoolQuery>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<StudentFormAssignmentResponse>, AppError> {
    use axum::extract::Multipart;

    check_permission_school_access(&auth, &q.school_id)?;

    let mut file_bytes = Vec::new();
    let mut file_name = String::new();
    let mut uploaded_by = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| AppError::Validation("Invalid multipart data".to_string()))? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                file_name = field.file_name().unwrap_or("document.pdf").to_string();
                file_bytes = field.bytes().await.map_err(|_| AppError::Validation("Failed to read file".to_string()))?.to_vec();
            }
            "uploaded_by" => {
                uploaded_by = String::from_utf8(field.bytes().await.map_err(|_| AppError::Validation("Failed to read uploaded_by".to_string()))?.to_vec())
                    .unwrap_or_else(|_| auth.email.clone());
            }
            _ => {}
        }
    }

    if file_bytes.is_empty() {
        return Err(AppError::Validation("No file provided".to_string()));
    }

    if uploaded_by.is_empty() {
        uploaded_by = auth.email.clone();
    }

    Ok(Json(svc.upload_manual_pdf(id, q.school_id, file_bytes, file_name, uploaded_by).await?))
}
