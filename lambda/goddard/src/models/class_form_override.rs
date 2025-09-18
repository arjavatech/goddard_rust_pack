use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClassFormOverride {
    pub id: Uuid,
    pub school_id: Uuid,
    pub classroom_id: Uuid,
    pub form_template_id: Uuid,
    pub action: Option<String>,
    pub is_required: Option<bool>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateClassFormOverrideRequest {
    pub school_id: Uuid,
    pub classroom_id: Uuid,
    pub form_template_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteClassFormOverrideParams {
    pub id: Uuid,
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ClassFormOverrideResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub classroom_id: Uuid,
    pub form_template_id: Uuid,
    pub action: Option<String>,
    pub is_required: Option<bool>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(serde::Serialize)]
pub struct DeleteClassFormOverrideResponse {
    pub message: String,
    pub id: Uuid,
    pub school_id: Uuid,
    pub classroom_id: Uuid,
    pub form_template_id: Uuid,
}

impl From<ClassFormOverride> for ClassFormOverrideResponse {
    fn from(override_obj: ClassFormOverride) -> Self {
        Self {
            id: override_obj.id,
            school_id: override_obj.school_id,
            classroom_id: override_obj.classroom_id,
            form_template_id: override_obj.form_template_id,
            action: override_obj.action,
            is_required: override_obj.is_required,
            is_active: override_obj.is_active,
            created_at: override_obj.created_at,
        }
    }
}