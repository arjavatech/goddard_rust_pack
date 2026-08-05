use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientMessage {
    pub action: String,  // "subscribe", "mark_read", "ping"
    pub notification_id: Option<Uuid>,
    pub filter: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMessage {
    pub message_type: String,  // "connected", "notification", "pong", "notification_list"
    pub unread_count: Option<i64>,
    pub data: Option<serde_json::Value>,
}

impl ServerMessage {
    pub fn connected(unread_count: i64) -> Self {
        Self {
            message_type: "connected".to_string(),
            unread_count: Some(unread_count),
            data: None,
        }
    }

    pub fn new_notification(notification: serde_json::Value) -> Self {
        Self {
            message_type: "new_notification".to_string(),
            unread_count: None,
            data: Some(notification),
        }
    }

    pub fn pong() -> Self {
        Self {
            message_type: "pong".to_string(),
            unread_count: None,
            data: None,
        }
    }
}
