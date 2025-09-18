use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
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