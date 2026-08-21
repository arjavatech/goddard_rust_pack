use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Newsletter {
    pub id: Uuid, pub school_id: Uuid, pub title: String, pub content_blocks: Value,
    pub rendered_html: String, pub audience_scope: String, pub classroom_ids: Vec<Uuid>,
    pub status: String, pub scheduled_at: Option<DateTime<Utc>>, pub school_timezone: String,
    pub reminder_offsets_days: Vec<i16>, pub published_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_children: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertNewsletterBody {
    pub school_id: Uuid, pub title: String, pub content_blocks: Value, pub rendered_html: String,
    pub audience_scope: String, #[serde(default)] pub classroom_ids: Vec<Uuid>,
    pub scheduled_at: Option<DateTime<Utc>>, pub school_timezone: Option<String>,
    #[serde(default)] pub reminder_offsets_days: Vec<i16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishNewsletterBody { pub scheduled_at: Option<DateTime<Utc>>, pub school_timezone: Option<String>, #[serde(default)] pub reminder_offsets_days: Vec<i16> }

#[derive(Debug, Deserialize)]
pub struct ListNewslettersQuery { pub school_id: Option<Uuid>, pub limit: Option<i64>, pub offset: Option<i64> }

#[derive(Debug, Serialize)]
pub struct NewsletterListResponse { pub items: Vec<Newsletter>, pub total: i64 }
