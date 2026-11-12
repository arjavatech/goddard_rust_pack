use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::presigning::PresigningConfig;
use crate::error::error_types::AppError;
use crate::models::upload::UploadImageResponse;
use std::time::Duration;
use reqwest;

const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "application/pdf",
];

pub const DOCUMENT_ALLOWED_CONTENT_TYPES: &[&str] = &["application/pdf", "image/jpeg", "image/png"];
pub const DOCUMENT_MAX_SIZE_BYTES: i64 = 10 * 1024 * 1024;

pub const DRIVE_MAX_SIZE_BYTES: i64 = 500 * 1024 * 1024; // 500 MB

fn resize_bill_image(bytes: Vec<u8>, content_type: &str) -> (Vec<u8>, String) {
    if content_type == "application/pdf" {
        return (bytes, content_type.to_string());
    }
    const MAX_DIM: u32 = 1920;
    const THRESHOLD: usize = 2 * 1024 * 1024;
    if bytes.len() <= THRESHOLD {
        return (bytes, content_type.to_string());
    }
    let img = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(_) => return (bytes, content_type.to_string()),
    };
    let img = if img.width() > MAX_DIM || img.height() > MAX_DIM {
        img.resize(MAX_DIM, MAX_DIM, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut output = std::io::Cursor::new(Vec::new());
    if img.write_to(&mut output, image::ImageFormat::Jpeg).is_err() {
        return (bytes, content_type.to_string());
    }
    (output.into_inner(), "image/jpeg".to_string())
}

pub struct UploadService {
    s3_client: Option<aws_sdk_s3::Client>,
    bucket: Option<String>,
    base_url: Option<String>,
}

impl UploadService {
    pub async fn new() -> Self {
        let bucket = std::env::var("S3_UPLOAD_BUCKET").ok();
        let base_url = std::env::var("S3_BASE_URL").ok();

        if bucket.is_some() && base_url.is_some() {
            let config = aws_config::load_from_env().await;
            let s3_client = aws_sdk_s3::Client::new(&config);
            UploadService { s3_client: Some(s3_client), bucket, base_url }
        } else {
            println!("[WARN] UploadService disabled - missing S3_UPLOAD_BUCKET or S3_BASE_URL");
            UploadService { s3_client: None, bucket: None, base_url: None }
        }
    }

    pub async fn upload_image(
        &self,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<UploadImageResponse, AppError> {
        // Normalize "image/jpg" (non-standard) to "image/jpeg"
        let content_type = if content_type == "image/jpg" { "image/jpeg" } else { content_type };
        if !ALLOWED_CONTENT_TYPES.contains(&content_type) {
            return Err(AppError::Validation(format!(
                "Content type '{}' is not allowed. Use JPEG, PNG, GIF, WebP, or PDF.",
                content_type
            )));
        }

        let client = self.s3_client.as_ref()
            .ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().unwrap();
        let base_url = self.base_url.as_ref().unwrap();

        let ext = file_name.rsplit('.').next().unwrap_or("jpg").to_lowercase();
        let safe_ext = match ext.as_str() {
            "jpg" | "jpeg" => "jpg",
            "png" => "png",
            "gif" => "gif",
            "webp" => "webp",
            "pdf" => "pdf",
            _ => "jpg",
        };
        let key = format!("product-images/{}.{}", uuid::Uuid::new_v4(), safe_ext);

        client
            .put_object()
            .bucket(bucket)
            .key(&key)
            .content_type(content_type)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 upload failed: {}", e)))?;

        let s3_url = format!("{}/{}", base_url.trim_end_matches('/'), key);
        Ok(UploadImageResponse { s3_url })
    }

    pub async fn create_document_upload_url(
        &self,
        key: &str,
        content_type: &str,
        file_size_bytes: i64,
    ) -> Result<String, AppError> {
        if !DOCUMENT_ALLOWED_CONTENT_TYPES.contains(&content_type) {
            return Err(AppError::Validation("Documents must be PDF, JPG/JPEG, or PNG".to_string()));
        }
        if file_size_bytes <= 0 || file_size_bytes > DOCUMENT_MAX_SIZE_BYTES {
            return Err(AppError::Validation("Document size must be between 1 byte and 10 MB".to_string()));
        }
        let client = self.s3_client.as_ref().ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().ok_or_else(|| AppError::Internal("S3 upload bucket not configured".to_string()))?;
        let config = PresigningConfig::expires_in(Duration::from_secs(300))
            .map_err(|e| AppError::Internal(format!("Failed to create upload expiry: {}", e)))?;
        let request = client.put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .content_length(file_size_bytes)
            .presigned(config)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create document upload URL: {}", e)))?;
        Ok(request.uri().to_string())
    }

    pub async fn create_drive_upload_url(
        &self,
        key: &str,
        content_type: &str,
        file_size_bytes: i64,
    ) -> Result<String, AppError> {
        if file_size_bytes <= 0 || file_size_bytes > DRIVE_MAX_SIZE_BYTES {
            return Err(AppError::Validation("File size must be between 1 byte and 500 MB".to_string()));
        }
        let client = self.s3_client.as_ref().ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().ok_or_else(|| AppError::Internal("S3 upload bucket not configured".to_string()))?;
        let config = PresigningConfig::expires_in(Duration::from_secs(3600))
            .map_err(|e| AppError::Internal(format!("Failed to create upload expiry: {}", e)))?;
        let request = client.put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .content_length(file_size_bytes)
            .presigned(config)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create drive upload URL: {}", e)))?;
        Ok(request.uri().to_string())
    }

    pub async fn verify_document_object(&self, key: &str, content_type: &str, file_size_bytes: i64) -> Result<(), AppError> {
        let client = self.s3_client.as_ref().ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().ok_or_else(|| AppError::Internal("S3 upload bucket not configured".to_string()))?;
        let object = client.head_object().bucket(bucket).key(key).send().await
            .map_err(|_| AppError::Validation("Uploaded document was not found or has expired".to_string()))?;
        if object.content_length().unwrap_or_default() != file_size_bytes || object.content_type().unwrap_or_default() != content_type {
            return Err(AppError::Validation("Uploaded document metadata does not match the approved upload request".to_string()));
        }
        Ok(())
    }

    pub async fn create_document_access_url(&self, key: &str, download: bool, filename: Option<&str>) -> Result<String, AppError> {
        let client = self.s3_client.as_ref().ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().ok_or_else(|| AppError::Internal("S3 upload bucket not configured".to_string()))?;
        let config = PresigningConfig::expires_in(Duration::from_secs(300))
            .map_err(|e| AppError::Internal(format!("Failed to create file access expiry: {}", e)))?;
        let disposition = if download {
            match filename {
                Some(name) => format!("attachment; filename=\"{}\"", name.replace('"', "\\\"")),
                None => "attachment".to_string(),
            }
        } else {
            "inline".to_string()
        };
        let request = client.get_object().bucket(bucket).key(key)
            .response_content_disposition(&disposition)
            .presigned(config).await
            .map_err(|e| AppError::Internal(format!("Failed to create document access URL: {}", e)))?;
        Ok(request.uri().to_string())
    }

    pub async fn delete_document_object(&self, key: &str) -> Result<(), AppError> {
        let client = self.s3_client.as_ref().ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().ok_or_else(|| AppError::Internal("S3 upload bucket not configured".to_string()))?;
        client.delete_object().bucket(bucket).key(key).send().await
            .map_err(|e| AppError::Internal(format!("Failed to remove S3 object: {}", e)))?;
        Ok(())
    }

    pub async fn copy_s3_object(&self, source_key: &str, dest_key: &str) -> Result<(), AppError> {
        let client = self.s3_client.as_ref().ok_or_else(|| AppError::Internal("S3 upload not configured".to_string()))?;
        let bucket = self.bucket.as_ref().ok_or_else(|| AppError::Internal("S3 upload bucket not configured".to_string()))?;
        let copy_source = format!("{}/{}", bucket, source_key);
        client.copy_object()
            .copy_source(copy_source)
            .bucket(bucket)
            .key(dest_key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to copy S3 object: {}", e)))?;
        Ok(())
    }

    pub async fn upload_bill_image(&self, file_name: &str, content_type: &str, bytes: Vec<u8>) -> Result<UploadImageResponse, AppError> {
        let (bytes, effective_ct) = resize_bill_image(bytes, content_type);
        self.upload_image(file_name, &effective_ct, bytes).await
    }

    pub async fn upload_document(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), AppError> {
        // Validate size
        if bytes.len() as i64 > DOCUMENT_MAX_SIZE_BYTES {
            return Err(AppError::Validation("File size exceeds 10 MB limit".to_string()));
        }

        // Generate presigned PUT URL using existing method
        let presigned_url = self.create_document_upload_url(key, content_type, bytes.len() as i64).await?;

        // PUT file to S3 via presigned URL using reqwest
        let client = reqwest::Client::new();
        let response = client
            .put(&presigned_url)
            .header("Content-Type", content_type)
            .header("Content-Length", bytes.len())
            .body(bytes)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 upload request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("S3 upload failed ({}): {}", status, body)));
        }

        Ok(())
    }
}
