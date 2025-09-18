use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormTemplate {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub fillout_form_url: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFormTemplateRequest {
    pub school_id: Uuid,
    pub form_name: String,
    pub fillout_form_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFormTemplateRequest {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub fillout_form_url: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFormTemplateParams {
    pub form_id: Uuid,
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct FormTemplateResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub fillout_form_url: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct FormTemplateListResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub form_name: String,
    pub form_type: Option<String>,
    pub fillout_form_id: Option<String>,
    pub fillout_form_url: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(serde::Serialize)]
pub struct DeleteFormTemplateResponse {
    pub message: String,
    pub form_id: Uuid,
    pub school_id: Uuid,
}

impl From<FormTemplate> for FormTemplateResponse {
    fn from(form_template: FormTemplate) -> Self {
        Self {
            id: form_template.id,
            school_id: form_template.school_id,
            form_name: form_template.form_name,
            form_type: form_template.form_type,
            fillout_form_id: form_template.fillout_form_id,
            fillout_form_url: form_template.fillout_form_url,
            status: form_template.status,
            is_required: form_template.is_required,
            display_order: form_template.display_order,
            created_at: form_template.created_at,
            updated_at: form_template.updated_at,
        }
    }
}

impl From<FormTemplate> for FormTemplateListResponse {
    fn from(form_template: FormTemplate) -> Self {
        Self {
            id: form_template.id,
            school_id: form_template.school_id,
            form_name: form_template.form_name,
            form_type: form_template.form_type,
            fillout_form_id: form_template.fillout_form_id,
            fillout_form_url: form_template.fillout_form_url,
            status: form_template.status,
            is_required: form_template.is_required,
            display_order: form_template.display_order,
            created_at: form_template.created_at,
        }
    }
}