use aws_sdk_s3::primitives::ByteStream;
use crate::error::error_types::AppError;
use crate::models::upload::UploadImageResponse;

const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
];

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
            println!("[DEBUG] UploadService initialized (bucket={})", bucket.as_deref().unwrap_or(""));
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
        if !ALLOWED_CONTENT_TYPES.contains(&content_type) {
            return Err(AppError::Validation(format!(
                "Content type '{}' is not allowed. Use JPEG, PNG, GIF, or WebP.",
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
}
