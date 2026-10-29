use std::sync::Arc;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::{AuthContext, check_permission_admin_or_superadmin};
use crate::models::conference::*;
use crate::services::conference_service::ConferenceService;

// ── Query param structs ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SchoolQuery {
    pub school_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct SlotsQuery {
    pub classroom_id: Uuid,
}

#[derive(Deserialize)]
pub struct MyConfsQuery {
    pub school_id: Uuid,
    pub child_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct MyBookingsQuery {
    pub school_id: Uuid,
}

// ── Admin: list ───────────────────────────────────────────────────────────────

pub async fn list_conferences(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<SchoolQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id.unwrap_or(auth.school_id);
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    Ok(Json(svc.list_conferences(school_id, &auth).await?))
}

// ── Admin: create ─────────────────────────────────────────────────────────────

pub async fn create_conference(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateConferenceRequest>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &payload.school_id)?;
    let item = svc.create_conference(payload, &auth).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

// ── Admin: stats ──────────────────────────────────────────────────────────────

pub async fn get_conference_stats(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<SchoolQuery>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id.unwrap_or(auth.school_id);
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    Ok(Json(svc.get_conference_stats(school_id, &auth).await?))
}

// ── Admin / any: get detail ───────────────────────────────────────────────────

pub async fn get_conference(
    State(svc): State<Arc<ConferenceService>>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.get_conference_detail(id).await?))
}

// ── Admin: update ─────────────────────────────────────────────────────────────

pub async fn update_conference(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(mut payload): Json<UpdateConferenceRequest>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &auth.school_id)?;
    payload.id = id;
    Ok(Json(svc.update_conference(payload, &auth).await?))
}

// ── Admin: delete ─────────────────────────────────────────────────────────────

pub async fn delete_conference(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &auth.school_id)?;
    svc.delete_conference(id, &auth).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Admin: report ─────────────────────────────────────────────────────────────

pub async fn get_conference_report(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &auth.school_id)?;
    Ok(Json(svc.get_conference_report(id, &auth).await?))
}

// ── Admin: send reminders ─────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct RemindRequest {
    pub parent_email: Option<String>,
}

pub async fn send_conference_reminders(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    body: Option<Json<RemindRequest>>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &auth.school_id)?;
    let parent_email = body.and_then(|b| b.0.parent_email);
    let count = svc.send_reminders(id, parent_email.as_deref(), &auth).await?;
    Ok(Json(serde_json::json!({ "reminders_sent": count })))
}

// ── Parent: list available ────────────────────────────────────────────────────

pub async fn list_my_conferences(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<MyConfsQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.list_available_for_parent(params.school_id, params.child_id, &auth).await?))
}

// ── Parent: get slots for a classroom ────────────────────────────────────────

pub async fn get_conference_slots(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(params): Query<SlotsQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.get_slots_for_classroom(id, params.classroom_id, &auth).await?))
}

// ── Parent: book slot ─────────────────────────────────────────────────────────

pub async fn book_slot(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<BookSlotRequest>,
) -> Result<impl IntoResponse, AppError> {
    let booking = svc.book_slot(payload, &auth).await?;
    Ok((StatusCode::CREATED, Json(booking)))
}

// ── Parent: list my bookings ──────────────────────────────────────────────────

pub async fn list_my_bookings(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<MyBookingsQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.list_my_bookings(params.school_id, &auth).await?))
}

// ── Parent: update booking (change slot) ─────────────────────────────────────

pub async fn update_booking(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBookingRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.update_booking(id, payload, &auth).await?))
}

// ── Parent: cancel booking ────────────────────────────────────────────────────

pub async fn cancel_booking(
    State(svc): State<Arc<ConferenceService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    svc.cancel_booking(id, &auth).await?;
    Ok(StatusCode::NO_CONTENT)
}
