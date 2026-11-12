use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Request DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DriveUploadUrlRequest {
    pub file_name: String,
    pub content_type: String,
    pub file_size: i64,
    pub folder_id: Option<Uuid>,
    pub drive_type: Option<String>, // 'admin' | 'owner'; service enforces based on role
}

#[derive(Debug, Deserialize)]
pub struct DriveRegisterFileRequest {
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub s3_key: String,
    pub mime_type: String,
    pub file_size: i64,
    pub drive_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DriveCreateFolderRequest {
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub drive_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DriveRenameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct DriveMoveRequest {
    pub folder_id: Option<Uuid>, // None = move to root
}

#[derive(Debug, Deserialize)]
pub struct DriveListQuery {
    pub folder_id: Option<Uuid>,
    pub drive_type: Option<String>, // 'admin' | 'owner'; service defaults based on role
    pub sort_by: Option<String>,    // name | file_size | created_at | updated_at
    pub sort_order: Option<String>, // asc | desc
}

#[derive(Debug, Deserialize)]
pub struct DriveDownloadQuery {
    pub download: Option<bool>,
}

// ── Response DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct DriveItem {
    pub id: Uuid,
    pub folder_id: Option<Uuid>,
    pub user_id: Uuid,
    pub drive_type: String,
    pub name: String,
    pub item_type: String,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub modified_by_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DriveUploadUrlResponse {
    pub upload_url: String,
    pub s3_key: String,
}

#[derive(Debug, Serialize)]
pub struct DriveDownloadUrlResponse {
    pub url: String,
}
