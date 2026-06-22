use std::sync::Arc;
use uuid::Uuid;

use crate::dao::NotificationDao;
use crate::error::AppError;
use crate::models::notification::{
    CreateNotification, NotificationFilter, NotificationListResponse,
};

/// Wraps the NotificationDao and provides fire-and-forget helpers used by the rest of the
/// service layer. See docs/IN_APP_NOTIFICATIONS.md.
pub struct NotificationService {
    dao: Arc<NotificationDao>,
}

impl NotificationService {
    pub fn new(dao: NotificationDao) -> Self {
        Self { dao: Arc::new(dao) }
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
    // Callers `.await` these directly. The DB insert runs inline (single SQL round trip),
    // so the notification row is guaranteed to exist before the API response is sent.
    // Errors are logged but never propagated — the originating API call still succeeds.
    //
    // Previously these used `tokio::spawn` for fire-and-forget. That broke under Lambda
    // (the runtime kills the worker the instant the handler returns, so the detached
    // task never gets to run) and could race even in local dev. The DB write is cheap
    // enough to do inline. HTTP-bound side-effects like Resend still use spawn.

    pub async fn notify_user(&self, user_id: Uuid, payload: CreateNotification) {
        match self.dao.insert_one(user_id, &payload).await {
            Ok(_) => {
                println!(
                    "[NotificationService] inserted notification (user={}, type={})",
                    user_id, payload.notification_type
                );
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
            Ok(n) => {
                println!(
                    "[NotificationService] fanned out {} admin notifications (type={}, school={})",
                    n, payload.notification_type, payload.school_id
                );
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
