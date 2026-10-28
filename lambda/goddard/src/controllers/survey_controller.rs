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
use crate::models::survey::*;
use crate::services::survey_service::SurveyService;

#[derive(Deserialize)]
pub struct SurveyQueryParams {
    pub school_id: Option<Uuid>,
    pub status: Option<String>,
    pub audience: Option<String>,
}

#[derive(Deserialize)]
pub struct MySurveysParams {
    pub school_id: Uuid,
}

// ── Admin: list all surveys ───────────────────────────────────────────────────

pub async fn list_surveys(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<SurveyQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id.unwrap_or(auth.school_id);
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    let query = SurveyListQuery { school_id, status: params.status, audience: params.audience };
    Ok(Json(svc.list_surveys(query, &auth).await?))
}

// ── Admin: create survey ──────────────────────────────────────────────────────

pub async fn create_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateSurveyRequest>,
) -> Result<impl IntoResponse, AppError> {
    check_permission_admin_or_superadmin(&auth, &payload.school_id)?;
    let detail = svc.create_survey(payload, &auth).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

// ── Admin: list templates ─────────────────────────────────────────────────────

pub async fn list_templates(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<SurveyQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let school_id = params.school_id.unwrap_or(auth.school_id);
    check_permission_admin_or_superadmin(&auth, &school_id)?;
    Ok(Json(svc.list_templates(&auth).await?))
}

// ── Any role: get survey detail ────────────────────────────────────────────────

pub async fn get_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.get_survey_detail(id, &auth).await?))
}

// ── Admin: update survey ──────────────────────────────────────────────────────

pub async fn update_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(mut payload): Json<UpdateSurveyRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload.id = id;
    Ok(Json(svc.update_survey(payload, &auth).await?))
}

// ── Admin: delete survey ──────────────────────────────────────────────────────

pub async fn delete_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    svc.delete_survey(id, &auth).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Admin: publish survey ─────────────────────────────────────────────────────

pub async fn publish_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.publish_survey(id, &auth).await?))
}

// ── Admin: close survey ───────────────────────────────────────────────────────

pub async fn close_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.close_survey(id, &auth).await?))
}

// ── Public: fetch anonymous survey (no auth) ─────────────────────────────────

pub async fn get_public_survey(
    State(svc): State<Arc<SurveyService>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.get_public_survey_detail(id).await?))
}

// ── Admin: reopen survey ─────────────────────────────────────────────────────

pub async fn reopen_survey(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.reopen_survey(id, &auth).await?))
}

// ── Admin: get target audience count ─────────────────────────────────────────

pub async fn get_target_count(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<TargetCountQuery>,
) -> Result<impl IntoResponse, AppError> {
    let count = svc.get_target_count(auth.school_id, &params.audience, params.classroom_id).await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

// ── Admin: survey report ──────────────────────────────────────────────────────

pub async fn get_survey_report(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.get_report(id, &auth).await?))
}

// ── Any role: list my surveys ─────────────────────────────────────────────────

pub async fn list_my_surveys(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<MySurveysParams>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(svc.list_my_surveys(&auth, params.school_id).await?))
}

// ── Authenticated: submit survey response ─────────────────────────────────────

pub async fn submit_survey_response(
    State(svc): State<Arc<SurveyService>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SubmitResponseRequest>,
) -> Result<impl IntoResponse, AppError> {
    svc.submit_response(id, payload, Some(auth.user_id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Public: anonymous survey response ────────────────────────────────────────

pub async fn submit_anonymous_response(
    State(svc): State<Arc<SurveyService>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AnonymousSubmitRequest>,
) -> Result<impl IntoResponse, AppError> {
    let detail = svc.dao_ref().get_survey_detail(id, None).await?
        .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))?;
    if !detail.survey.is_anonymous {
        return Err(AppError::Authorization("This survey requires authentication".to_string()));
    }
    let full_req = SubmitResponseRequest {
        school_id: detail.survey.school_id,
        answers: payload.answers,
    };
    svc.submit_response(id, full_req, None).await?;
    Ok(StatusCode::NO_CONTENT)
}
