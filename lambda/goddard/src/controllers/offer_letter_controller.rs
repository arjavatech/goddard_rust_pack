use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::error_types::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::offer_letter::{
    AdminReviewRequest, CreateOfferLetterRequest, OfferLetterWebhookPayload,
    UpdateOfferLetterRequest,
};
use crate::services::offer_letter_service::OfferLetterService;

#[derive(Deserialize)]
pub struct SchoolIdQuery {
    pub school_id: Uuid,
}

// ─── Admin: Create offer letter ───────────────────────────────────────────────

pub async fn create_offer_letter(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Json(payload): Json<CreateOfferLetterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let offer = svc.create_offer_letter(employee_id, payload).await?;
    Ok((StatusCode::CREATED, Json(offer)))
}

// ─── Admin: Get offer letter ──────────────────────────────────────────────────

pub async fn get_offer_letter(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let offer = svc.get_offer_letter(employee_id, params.school_id).await?;
    Ok((StatusCode::OK, Json(offer)))
}

// ─── Admin: Update offer letter (draft only) ─────────────────────────────────

pub async fn update_offer_letter(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
    Json(payload): Json<UpdateOfferLetterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let offer = svc.update_offer_letter(employee_id, params.school_id, payload).await?;
    Ok((StatusCode::OK, Json(offer)))
}

// ─── Admin: Send offer letter ─────────────────────────────────────────────────

pub async fn send_offer_letter(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let offer = svc.send_offer_letter(employee_id, params.school_id).await?;
    Ok((StatusCode::OK, Json(offer)))
}

// ─── Admin: Resend offer letter ───────────────────────────────────────────────

pub async fn resend_offer_letter(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let offer = svc.resend_offer_letter(employee_id, params.school_id).await?;
    Ok((StatusCode::OK, Json(offer)))
}

// ─── Admin: Review / approve offer letter ────────────────────────────────────

pub async fn review_offer_letter(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(payload): Json<AdminReviewRequest>,
) -> Result<impl IntoResponse, AppError> {
    let offer = svc.admin_review(employee_id, params.school_id, payload, auth.user_id).await?;
    Ok((StatusCode::OK, Json(offer)))
}

// ─── Get offer letter history for employee ────────────────────────────────────

pub async fn get_offer_letter_history(
    State(svc): State<Arc<OfferLetterService>>,
    Path(employee_id): Path<Uuid>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let history = svc.get_offer_letter_history(employee_id, params.school_id).await?;
    Ok((StatusCode::OK, Json(history)))
}

// ─── Admin: List offer letters for school ─────────────────────────────────────

pub async fn list_offer_letters(
    State(svc): State<Arc<OfferLetterService>>,
    Query(params): Query<SchoolIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let letters = svc.list_for_school(params.school_id).await?;
    Ok((StatusCode::OK, Json(letters)))
}

// ─── Fillout webhook (unauthenticated — API key validation in handler) ────────

pub async fn offer_letter_webhook(
    State(svc): State<Arc<OfferLetterService>>,
    headers: HeaderMap,
    Json(payload): Json<OfferLetterWebhookPayload>,
) -> Result<impl IntoResponse, AppError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Authentication("Missing X-API-Key header".to_string()))?;

    let expected = std::env::var("OWNER_API_KEY")
        .map_err(|_| AppError::Internal("OWNER_API_KEY not configured".to_string()))?;

    if api_key != expected {
        return Err(AppError::Authentication("Invalid API key".to_string()));
    }

    let offer = svc.handle_fillout_webhook(payload).await?;
    Ok((StatusCode::OK, Json(offer)))
}
