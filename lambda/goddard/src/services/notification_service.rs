use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{NotificationDao, SchoolDao};
use crate::error::AppError;
use crate::models::notification::{
    CreateNotification, NotificationFilter, NotificationListResponse,
};
use crate::services::NotificationPushTrigger;

/// Wraps the NotificationDao and records notification work transactionally for the rest of the
/// service layer. Eligible browser pushes are queued durably for the scheduled FCM worker.
pub struct NotificationService {
    dao: Arc<NotificationDao>,
    school_dao: SchoolDao,
    push_trigger: Option<Arc<NotificationPushTrigger>>,
}

impl NotificationService {
    pub fn new(
        dao: NotificationDao,
        school_dao: SchoolDao,
        push_trigger: Option<NotificationPushTrigger>,
    ) -> Self {
        Self {
            dao: Arc::new(dao),
            school_dao,
            push_trigger: push_trigger.map(Arc::new),
        }
    }

    /// Notification routes must always include the school's public subdomain.
    /// Producers intentionally supply application-relative paths (for example
    /// `/admin/forms/review`); resolving them here keeps every delivery channel
    /// (bell, FCM foreground, and service-worker click) on the same route.
    async fn with_school_scoped_action(&self, mut payload: CreateNotification) -> CreateNotification {
        let Some(action_url) = payload.action_url.clone() else {
            return payload;
        };

        // Only scope internal, root-relative application routes. External URLs
        // remain untouched should they be introduced in a future notification.
        if !action_url.starts_with('/') || action_url.starts_with("//") {
            return payload;
        }

        match self.school_dao.get_school_by_id(&payload.school_id).await {
            Ok(Some(school)) if !school.subdomain.trim().is_empty() => {
                let trimmed_path = action_url.trim_start_matches('/');
                let slug = school.subdomain.trim();
                if trimmed_path != slug && !trimmed_path.starts_with(&format!("{slug}/")) {
                    payload.action_url = Some(format!("/{slug}/{trimmed_path}"));
                }
            }
            Ok(Some(_)) => eprintln!(
                "[NotificationService] school {} has no subdomain; leaving action URL unscoped",
                payload.school_id
            ),
            Ok(None) => eprintln!(
                "[NotificationService] school {} not found; leaving action URL unscoped",
                payload.school_id
            ),
            Err(error) => eprintln!(
                "[NotificationService] unable to resolve school route for {}: {:?}",
                payload.school_id, error
            ),
        }

        payload
    }

    // ---- Read APIs (used by controller) ----

    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: NotificationFilter,
        limit: i64,
        offset: i64,
    ) -> Result<NotificationListResponse, AppError> {
        let (items, total, unread_count) = self
            .dao
            .list_for_user(user_id, filter, limit, offset)
            .await?;
        Ok(NotificationListResponse {
            items,
            total,
            unread_count,
        })
    }

    pub async fn count_unread(&self, user_id: Uuid) -> Result<i64, AppError> {
        self.dao.count_unread(user_id).await
    }

    pub async fn mark_read(&self, notification_id: Uuid, user_id: Uuid) -> Result<bool, AppError> {
        self.dao.mark_read(notification_id, user_id).await
    }

    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, AppError> {
        self.dao.mark_all_read(user_id).await
    }

    // ---- Notification delivery helpers (used by sibling services) ----
    // Notification rows remain the source of truth for the in-app bell. Browser push is
    // deliberately limited to action-required events; informational events stay in-app.
    fn should_send_push(notification_type: &str) -> bool {
        matches!(
            notification_type,
            "form_assigned"
                | "form_submitted"
                | "form_approved"
                | "form_rejected"
                | "document_requested"
                | "document_submitted"
                | "document_approved"
                | "document_rejected"
        )
    }

    /// Wake the outbox worker without adding an AWS invocation round-trip to
    /// the request that generated the notification. The scheduled worker is
    /// still the durable recovery path if this best-effort wake-up fails.
    fn wake_push_worker(&self) {
        if let Some(trigger) = self.push_trigger.clone() {
            tokio::spawn(async move {
                trigger.wake().await;
            });
        }
    }

    pub async fn notify_user(&self, user_id: Uuid, payload: CreateNotification) {
        let payload = self.with_school_scoped_action(payload).await;
        let enqueue_push = Self::should_send_push(&payload.notification_type);
        match self.dao.insert_one(user_id, &payload, enqueue_push).await {
            Ok(notification) => {
                println!(
                    "[NotificationService] inserted notification (user={}, type={})",
                    user_id, payload.notification_type
                );

                if enqueue_push {
                    println!(
                        "[NotificationService] queued push delivery (notification={})",
                        notification.id
                    );
                    self.wake_push_worker();
                }
            }
            Err(e) => {
                eprintln!(
                    "[NotificationService] notify_user failed (non-fatal): {:?}",
                    e
                );
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
        let payload = self.with_school_scoped_action(payload).await;
        let enqueue_push = Self::should_send_push(&payload.notification_type);
        match self
            .dao
            .insert_many_for_school_admins(
                payload.school_id,
                &payload,
                exclude_user_id,
                enqueue_push,
            )
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

                if enqueue_push {
                    println!(
                        "[NotificationService] queued {} admin push deliveries",
                        recipients.len()
                    );
                    self.wake_push_worker();
                }
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
