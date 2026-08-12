use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
};
use std::sync::Arc;
use crate::services::upload_service::UploadService;
use crate::utils::response::ResponseUtils;
use crate::error::error_types::AppError;

pub async fn upload_image(
    State(service): State<Arc<UploadService>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut file_name = String::from("image.jpg");
    let mut content_type = String::from("image/jpeg");
    let mut bytes: Vec<u8> = Vec::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::Validation(format!("Multipart error: {}", e)))?
    {
        if field.name() == Some("file") {
            if let Some(name) = field.file_name() {
                file_name = name.to_string();
            }
            if let Some(ct) = field.content_type() {
                content_type = ct.to_string();
            }
            bytes = field.bytes().await
                .map_err(|e| AppError::Validation(format!("Failed to read file: {}", e)))?
                .to_vec();
        }
    }

    if bytes.is_empty() {
        return Err(AppError::Validation("No file provided in request".to_string()));
    }

    let result = service.upload_image(&file_name, &content_type, bytes).await?;
    Ok(ResponseUtils::success(result))
}
