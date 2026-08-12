use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct UploadImageResponse {
    pub s3_url: String,
}
