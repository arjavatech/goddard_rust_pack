use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Discriminator for `notifications.notification_type`. Stored as TEXT in PG so the FE can
/// add types via a code-only change. See docs/IN_APP_NOTIFICATIONS.md.
pub mod notification_type {
    pub const FORM_APPROVED: &str = "form_approved";
    pub const FORM_REJECTED: &str = "form_rejected";
    pub const FORM_ASSIGNED: &str = "form_assigned";
    pub const FORM_SUBMITTED: &str = "form_submitted";
    pub const CHILD_ADDED: &str = "child_added";
    pub const CHILD_ARCHIVED: &str = "child_archived";
    pub const PARENT_INVITED: &str = "parent_invited";
    pub const PARENT_DEACTIVATED: &str = "parent_deactivated";
    pub const ADMIN_ADDED: &str = "admin_added";
    pub const CLASSROOM_ADDED: &str = "classroom_added";
    pub const CLASSROOM_DELETED: &str = "classroom_deleted";
    pub const FORM_TEMPLATE_ADDED: &str = "form_template_added";
    pub const FORM_TEMPLATE_DELETED: &str = "form_template_deleted";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub related_entity_id: Option<Uuid>,
    pub related_entity_type: Option<String>,
    pub action_url: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Payload used by services to fire a notification. `user_id` is populated by the DAO when
/// fanning out to multiple recipients (e.g. all admins of a school).
#[derive(Debug, Clone)]
pub struct CreateNotification {
    pub school_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub related_entity_id: Option<Uuid>,
    pub related_entity_type: Option<String>,
    pub action_url: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationFilter {
    All,
    Unread,
    Read,
}

impl NotificationFilter {
    pub fn from_query(value: Option<&str>) -> Self {
        match value.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("unread") => Self::Unread,
            Some("read") => Self::Read,
            _ => Self::All,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub filter: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub items: Vec<Notification>,
    pub total: i64,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct MarkAllReadResponse {
    pub updated: i64,
}
