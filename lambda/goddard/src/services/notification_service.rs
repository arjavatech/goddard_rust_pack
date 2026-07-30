use std::sync::Arc;
use uuid::Uuid;

use crate::dao::NotificationDao;
use crate::error::AppError;
use crate::models::notification::{
    CreateNotification, NotificationFilter, NotificationListResponse,
};
use crate::services::{FcmService, ConnectionRegistry};

/// Wraps the NotificationDao and provides fire-and-forget helpers used by the rest of the
/// service layer. See docs/IN_APP_NOTIFICATIONS.md.
pub struct NotificationService {
    dao: Arc<NotificationDao>,
    fcm: Arc<FcmService>,
    registry: Arc<ConnectionRegistry>,
}

impl NotificationService {
    pub fn new(dao: NotificationDao, fcm: Arc<FcmService>, registry: Arc<ConnectionRegistry>) -> Self {
        Self {
            dao: Arc::new(dao),
            fcm,
            registry,
        }
    }

    // ---- Read APIs (used by controller) ----

    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: NotificationFilter,
        limit: i64,
        offset: i64,
    ) -> Result<NotificationListResponse, AppError> {
        let (items, total, unread_count) =
            self.dao.list_for_user(user_id, filter, limit, offset).await?;
        Ok(NotificationListResponse {
            items,
            total,
            unread_count,
        })
    }

    pub async fn count_unread(&self, user_id: Uuid) -> Result<i64, AppError> {
        self.dao.count_unread(user_id).await
    }

    /// Public method for WebSocket handler to fetch unread count
    pub async fn count_unread_ws(&self, user_id: Uuid) -> Result<i64, AppError> {
        self.count_unread(user_id).await
    }

    pub async fn mark_read(
        &self,
        notification_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AppError> {
        self.dao.mark_read(notification_id, user_id).await
    }

    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, AppError> {
        self.dao.mark_all_read(user_id).await
    }

    // ---- Synchronous fire helpers (used by sibling services) ----
    //
    // The DB insert runs inline (single SQL round trip), so the notification row is
    // guaranteed to exist before the API response is sent. The HTTP-bound FCM dispatch
    // is detached via `tokio::spawn` after the DB write — Lambda-safe because the row
    // is already durable by the time the API handler returns; the OS push is best-effort.
    //
    // Errors at either layer are logged but never propagated — the originating API call
    // still succeeds.

    /// Broadcast a notification to a user via WebSocket if they're connected
    async fn broadcast_to_websocket(&self, user_id: Uuid, notification: &crate::models::notification::Notification) {
        let msg = serde_json::json!({
            "id": notification.id,
            "notification_type": notification.notification_type,
            "title": notification.title,
            "body": notification.body,
            "is_read": notification.is_read,
            "created_at": notification.created_at,
            "related_entity_id": notification.related_entity_id,
            "related_entity_type": notification.related_entity_type,
            "action_url": notification.action_url,
        });

        let server_msg = crate::models::websocket_message::ServerMessage::new_notification(msg);
        let json_msg = serde_json::to_string(&server_msg).unwrap_or_default();

        let sent = self.registry.send_to_user(user_id, json_msg).await;
        if sent {
            println!(
                "[NotificationService] Broadcast to WebSocket for user: {} (type={})",
                user_id, notification.notification_type
            );
        }
    }

    pub async fn notify_user(&self, user_id: Uuid, payload: CreateNotification) {
        match self.dao.insert_one(user_id, &payload).await {
            Ok(notification) => {
                println!(
                    "[NotificationService] inserted notification (user={}, type={})",
                    user_id, payload.notification_type
                );

                // ✅ NEW: Broadcast immediately via WebSocket
                self.broadcast_to_websocket(user_id, &notification).await;

                let fcm = self.fcm.clone();
                let title = payload.title.clone();
                let body = payload.body.clone();
                let action_url = payload.action_url.clone();
                let related_id = payload.related_entity_id;
                let ntype = payload.notification_type.clone();
                tokio::spawn(async move {
                    fcm.send_to_user(
                        user_id,
                        &title,
                        &body,
                        action_url.as_deref(),
                        related_id,
                        &ntype,
                    )
                    .await;
                });
            }
            Err(e) => {
                eprintln!("[NotificationService] notify_user failed (non-fatal): {:?}", e);
            }
        }
    }

    /// Fan out one notification per active admin / superadmin of the school.
    /// `exclude_user_id` lets you skip a single recipient (e.g. the admin who initiated
    /// the action, so they don't get notified about their own change).
    pub async fn notify_school_admins(
        &self,
        payload: CreateNotification,
        exclude_user_id: Option<Uuid>,
    ) {
        match self
            .dao
            .insert_many_for_school_admins(payload.school_id, &payload, exclude_user_id)
            .await
        {
            Ok(recipients) => {
                println!(
                    "[NotificationService] fanned out {} admin notifications (type={}, school={})",
                    recipients.len(),
                    payload.notification_type,
                    payload.school_id
                );

                if recipients.is_empty() {
                    return;
                }

                // ✅ NEW: Broadcast to each admin's WebSocket
                // Fetch the created notifications to broadcast
                for recipient_id in &recipients {
                    // For each recipient, we need to fetch their notification to broadcast
                    if let Ok((notifications, _, _)) = self.dao.list_for_user(*recipient_id, NotificationFilter::All, 1, 0).await {
                        if let Some(notif) = notifications.first() {
                            self.broadcast_to_websocket(*recipient_id, notif).await;
                        }
                    }
                }

                let fcm = self.fcm.clone();
                let title = payload.title.clone();
                let body = payload.body.clone();
                let action_url = payload.action_url.clone();
                let related_id = payload.related_entity_id;
                let ntype = payload.notification_type.clone();
                tokio::spawn(async move {
                    fcm.send_to_users(
                        &recipients,
                        &title,
                        &body,
                        action_url.as_deref(),
                        related_id,
                        &ntype,
                    )
                    .await;
                });
            }
            Err(e) => {
                eprintln!(
                    "[NotificationService] notify_school_admins failed (non-fatal): {:?}",
                    e
                );
            }
        }
    }
}
