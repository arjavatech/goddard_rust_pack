use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::auth::AuthContext,
    services::{CreateMappingRequest, TapTimeMappingService},
    services::taptime_mapping_service::{RedeemPairingCodeRequest, TapTimeAccessSyncResult, TapTimeIntegrationStatus, TapTimeReconciliationResult, TapTimeSetupStatus, TapTimeSettingsResponse, UpdateTapTimeSettingsRequest},
};

#[derive(Deserialize)]
pub struct QueryParams {
    pub school_id: Uuid,
}

#[derive(Serialize)]
pub struct MappingUsersResponse {
    pub items: Vec<crate::services::taptime_mapping_service::MappingUser>,
}

#[derive(Serialize)]
pub struct AttendanceUsersResponse {
    pub items: Vec<crate::services::taptime_mapping_service::AttendanceUser>,
}

pub async fn mapping_users(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<MappingUsersResponse>, AppError> {
    Ok(Json(MappingUsersResponse {
        items: service.users(query.school_id).await?,
    }))
}

pub async fn attendance_users(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<AttendanceUsersResponse>, AppError> {
    Ok(Json(AttendanceUsersResponse {
        items: service.attendance_users(query.school_id).await?,
    }))
}

#[derive(Serialize)]
pub struct AvailableUsersResponse {
    pub items: Vec<serde_json::Value>,
}

pub async fn available_taptime_users(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<AvailableUsersResponse>, AppError> {
    Ok(Json(AvailableUsersResponse {
        items: service.available_taptime_users(query.school_id).await?,
    }))
}

pub async fn setup_status(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<TapTimeSetupStatus>, AppError> {
    Ok(Json(service.setup_status(query.school_id).await?))
}

pub async fn integration_status(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<TapTimeIntegrationStatus>, AppError> {
    Ok(Json(service.integration_status(query.school_id).await?))
}

pub async fn redeem_pairing_code(
    State(service): State<Arc<TapTimeMappingService>>,
    Json(request): Json<RedeemPairingCodeRequest>,
) -> Result<Json<TapTimeSetupStatus>, AppError> {
    Ok(Json(service.redeem_pairing_code(request).await?))
}

pub async fn sync_access(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<TapTimeAccessSyncResult>, AppError> {
    Ok(Json(service.sync_access(query.school_id).await?))
}

pub async fn reconcile_users(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<TapTimeReconciliationResult>, AppError> {
    Ok(Json(service.reconcile_with_summary(query.school_id).await?))
}

pub async fn taptime_settings(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
) -> Result<Json<TapTimeSettingsResponse>, AppError> {
    Ok(Json(service.settings(query.school_id).await?))
}

pub async fn update_taptime_settings(
    State(service): State<Arc<TapTimeMappingService>>,
    Query(query): Query<QueryParams>,
    Json(request): Json<UpdateTapTimeSettingsRequest>,
) -> Result<Json<TapTimeSettingsResponse>, AppError> {
    Ok(Json(service.update_settings(query.school_id, request).await?))
}

/// TEMPORARY: SuperAdmin-only live database identity check. Remove after the
/// development connection mismatch has been resolved.
pub async fn database_diagnostics(
    State(service): State<Arc<TapTimeMappingService>>,
) -> Result<Json<crate::dao::TapTimeDatabaseDiagnostics>, AppError> {
    let diagnostics = service.database_diagnostics().await?;
    tracing::info!(
        database = %diagnostics.current_database,
        user = %diagnostics.current_user,
        schema = %diagnostics.current_schema,
        server = %diagnostics.server_address,
        relation = ?diagnostics.resolved_relation,
        has_goddard_user_id = diagnostics.has_goddard_user_id,
        "TapTime mapping database diagnostics"
    );
    Ok(Json(diagnostics))
}

#[derive(Serialize)]
pub struct MappingCreatedResponse {
    pub status: &'static str,
}

pub async fn create_mapping(
    State(service): State<Arc<TapTimeMappingService>>,
    actor: Option<Extension<AuthContext>>,
    Json(request): Json<CreateMappingRequest>,
) -> Result<Json<MappingCreatedResponse>, AppError> {
    // API-key use is retained for trusted operational tooling. Browser requests
    // carry the SuperAdmin context installed by route middleware.
    let mapped_by = actor
        .map(|Extension(context)| context.user_id)
        .unwrap_or_else(Uuid::nil);
    service.create_mapping(request, mapped_by).await?;
    Ok(Json(MappingCreatedResponse { status: "mapped" }))
}
