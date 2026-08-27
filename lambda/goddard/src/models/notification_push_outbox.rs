use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NotificationPushJob {
    pub id: Uuid,
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub device_token: String,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub action_url: Option<String>,
}
