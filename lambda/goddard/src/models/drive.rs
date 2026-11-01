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
}

#[derive(Debug, Deserialize)]
pub struct DriveRegisterFileRequest {
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub s3_key: String,
    pub mime_type: String,
    pub file_size: i64,
}

#[derive(Debug, Deserialize)]
pub struct DriveCreateFolderRequest {
    pub folder_id: Option<Uuid>,
    pub name: String,
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
    pub user_id: Option<Uuid>,      // SuperAdmin uses this to browse an admin's drive
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
    pub name: String,
    pub item_type: String,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

#[derive(Debug, Serialize)]
pub struct DriveAdminSummary {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub item_count: i64,
}
