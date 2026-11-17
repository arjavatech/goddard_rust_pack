use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OfferLetter {
    pub id: Uuid,
    pub school_id: Uuid,
    pub employee_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub custom_fields: Option<Value>,
    pub fillout_form_id: Option<String>,
    pub fillout_submission_id: Option<String>,
    pub fillout_edit_link: Option<String>,
    pub fillout_pdf_link: Option<String>,
    pub form_data: Option<Value>,
    pub prefill_url: Option<String>,
    pub status: String,
    pub employee_responded_at: Option<NaiveDateTime>,
    pub employee_rejection_reason: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub admin_reviewed_at: Option<NaiveDateTime>,
    pub admin_rejection_reason: Option<String>,
    pub sent_at: Option<NaiveDateTime>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub is_current: bool,
    pub employee_submitted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOfferLetterRequest {
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub custom_fields: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOfferLetterRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub custom_fields: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct AdminReviewRequest {
    pub approved: bool,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OfferLetterWebhookPayload {
    pub offer_letter_id: Uuid,
    pub fillout_submission_id: String,
    pub accepted: Option<bool>,
    pub form_status: Option<String>,
    pub form_data: Option<Value>,
    pub edit_link: Option<String>,
    pub pdf_link: Option<String>,
    pub rejection_reason: Option<String>,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
}
