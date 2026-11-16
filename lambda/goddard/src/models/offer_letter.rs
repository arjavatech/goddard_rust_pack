use chrono::{NaiveDate, NaiveDateTime};
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
    pub role: String,
    pub salary: Option<f64>,
    pub salary_currency: Option<String>,
    pub joining_date: Option<NaiveDate>,
    pub additional_notes: Option<String>,
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
}

#[derive(Debug, Deserialize)]
pub struct CreateOfferLetterRequest {
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
    pub salary: Option<f64>,
    pub salary_currency: Option<String>,
    pub joining_date: Option<NaiveDate>,
    pub additional_notes: Option<String>,
    pub custom_fields: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOfferLetterRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    pub salary: Option<f64>,
    pub salary_currency: Option<String>,
    pub joining_date: Option<NaiveDate>,
    pub additional_notes: Option<String>,
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
    pub accepted: bool,
    pub form_data: Option<Value>,
    pub edit_link: Option<String>,
    pub pdf_link: Option<String>,
    pub rejection_reason: Option<String>,
}
