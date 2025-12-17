use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime, DateTime, Utc};
use uuid::Uuid;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct School {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
    pub settings: Option<Value>,
    pub is_active: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSchoolRequest {
    pub name: String,
    pub subdomain: String,
    pub settings: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSchoolRequest {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
    pub settings: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct SchoolResponse {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
    pub settings: Option<Value>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct SchoolListItem {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteSchoolResponse {
    pub message: String,
    pub school_id: Uuid,
}

impl From<School> for SchoolResponse {
    fn from(school: School) -> Self {
        Self {
            id: school.id,
            name: school.name,
            subdomain: school.subdomain,
            settings: school.settings,
            created_at: school.created_at,
            updated_at: school.updated_at,
        }
    }
}

impl From<School> for SchoolListItem {
    fn from(school: School) -> Self {
        Self {
            id: school.id,
            name: school.name,
            subdomain: school.subdomain,
        }
    }
}

// Models for creating school with owner endpoint

#[derive(Debug, Deserialize)]
pub struct SchoolInfo {
    pub name: String,
    pub subdomain: String,
    pub settings: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct OwnerInfo {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSchoolWithOwnerRequest {
    pub school: SchoolInfo,
    pub owner: OwnerInfo,
}

#[derive(Debug, Serialize)]
pub struct OwnerCreatedResponse {
    pub user_id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub email_sent: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CreateSchoolWithOwnerResponse {
    pub school: SchoolResponse,
    pub owner: OwnerCreatedResponse,
}

// Models for GET school with owner endpoint

#[derive(Debug, Serialize)]
pub struct OwnerDetailsResponse {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
    pub role: String,
    pub is_verified: bool,
    pub has_logged_in: bool,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SchoolWithOwnerResponse {
    pub school: SchoolResponse,
    pub owner: OwnerDetailsResponse,
}