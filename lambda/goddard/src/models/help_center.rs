use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SubmitHelpMessageRequest {
    pub sender_name: String,
    pub sender_email: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HelpCenterMessage {
    pub id: Uuid,
    pub school_id: Uuid,
    pub sender_user_id: Option<Uuid>,
    pub sender_role: Option<String>,
    pub sender_name: String,
    pub sender_email: String,
    pub subject: String,
    pub message: String,
    pub to_email: String,
    pub from_email: String,
    pub created_at: DateTime<Utc>,
}
