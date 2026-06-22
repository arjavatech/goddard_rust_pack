use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::auth::AuthContext,
    models::notification::{
        ListNotificationsQuery, MarkAllReadResponse, NotificationFilter, UnreadCountResponse,
    },
    services::NotificationService,
    utils::ResponseUtils,
};

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

/// GET /notifications?filter=all|unread|read&limit=20&offset=0
pub async fn list_notifications(
    Extension(auth): Extension<AuthContext>,
    State(service): State<Arc<NotificationService>>,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let filter = NotificationFilter::from_query(query.filter.as_deref());
    let limit = query
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);
    let offset = query.offset.unwrap_or(0).max(0);

    println!(
        "[NotificationController] list user={} filter={:?} limit={} offset={}",
        auth.user_id, filter, limit, offset
    );

    let response = service
        .list_for_user(auth.user_id, filter, limit, offset)
        .await?;
    Ok(ResponseUtils::success(response))
}

/// GET /notifications/unread-count
pub async fn unread_count(
    Extension(auth): Extension<AuthContext>,
    State(service): State<Arc<NotificationService>>,
) -> Result<impl IntoResponse, AppError> {
    let count = service.count_unread(auth.user_id).await?;
    Ok(ResponseUtils::success(UnreadCountResponse { count }))
}

/// PATCH /notifications/:id/read
pub async fn mark_read(
    Extension(auth): Extension<AuthContext>,
    State(service): State<Arc<NotificationService>>,
    Path(notification_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.mark_read(notification_id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /notifications/mark-all-read
pub async fn mark_all_read(
    Extension(auth): Extension<AuthContext>,
    State(service): State<Arc<NotificationService>>,
) -> Result<impl IntoResponse, AppError> {
    let updated = service.mark_all_read(auth.user_id).await? as i64;
    Ok(ResponseUtils::success(MarkAllReadResponse { updated }))
}
