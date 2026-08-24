use std::sync::Arc;

use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::error_types::AppError,
    middleware::auth::AuthContext,
    models::tap_time::{ConfirmReconciliationRequest, ConnectTapTimeRequest, CreateTimeReportRequest, ReconciliationProposal, SetMyTapTimePinRequest, SetTapTimePinRequest, SetTapTimeUserPinRequest, TapTimeConsolidatedReport, TapTimeConsolidatedReportSetting, TapTimeIntegrationDashboard, TapTimeReport, TapTimeReportPerson, TapTimeReportSetting, TimeAttendanceQuery, TimeAttendanceSummaryQuery, UpdateTapTimeConsolidatedReportSettingRequest, UpdateTimeReportRequest, UpsertTimeReportSettingRequest},
    services::TapTimeService,
};

#[derive(Deserialize)]
pub struct SchoolQuery { pub school_id: Uuid }

pub fn no_store_pin(response: crate::models::tap_time::TapTimePinResponse) -> (axum::http::HeaderMap, Json<crate::models::tap_time::TapTimePinResponse>) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    (headers, Json(response))
}

pub async fn get_tap_time_connection(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Query(query): Query<SchoolQuery>,
) -> Result<(StatusCode, Json<Option<crate::models::tap_time::TapTimeConnection>>), AppError> {
    Ok((StatusCode::OK, Json(service.get_connection(&auth, query.school_id).await?)))
}

pub async fn connect_tap_time(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(request): Json<ConnectTapTimeRequest>,
) -> Result<(StatusCode, Json<crate::models::tap_time::TapTimeConnection>), AppError> {
    Ok((StatusCode::CREATED, Json(service.connect(&auth, request).await?)))
}

pub async fn disconnect_tap_time(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path(school_id): Path<Uuid>,
) -> Result<(StatusCode, Json<crate::models::tap_time::TapTimeConnection>), AppError> {
    Ok((StatusCode::OK, Json(service.disconnect(&auth, school_id).await?)))
}

pub async fn retry_tap_time_sync(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path(school_id): Path<Uuid>,
) -> Result<(StatusCode, Json<crate::services::tap_time_service::SyncDispatchResponse>), AppError> {
    Ok((StatusCode::OK, Json(service.dispatch_employee_sync(&auth, school_id).await?)))
}

pub async fn set_employee_tap_time_pin(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path(employee_id): Path<Uuid>,
    Json(request): Json<SetTapTimePinRequest>,
) -> Result<StatusCode, AppError> {
    service.set_employee_pin(&auth, employee_id, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_employee_tap_time_pin(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(employee_id): Path<Uuid>, Query(query): Query<SchoolQuery>,
) -> Result<(axum::http::HeaderMap, Json<crate::models::tap_time::TapTimePinResponse>), AppError> { Ok(no_store_pin(service.employee_pin(&auth, employee_id, query.school_id).await?)) }

pub async fn set_my_tap_time_pin(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Json(request): Json<SetMyTapTimePinRequest>,
) -> Result<StatusCode, AppError> {
    service.set_my_pin(&auth, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_my_tap_time_pin(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>,
) -> Result<(axum::http::HeaderMap, Json<crate::models::tap_time::TapTimePinResponse>), AppError> { Ok(no_store_pin(service.my_pin(&auth).await?)) }

pub async fn set_admin_tap_time_pin(
    State(service): State<Arc<TapTimeService>>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<SetTapTimeUserPinRequest>,
) -> Result<StatusCode, AppError> {
    service.set_admin_pin(&auth, user_id, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_admin_tap_time_pin(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(user_id): Path<Uuid>, Query(query): Query<SchoolQuery>,
) -> Result<(axum::http::HeaderMap, Json<crate::models::tap_time::TapTimePinResponse>), AppError> { Ok(no_store_pin(service.admin_pin(&auth, user_id, query.school_id).await?)) }

pub async fn get_tap_time_pins(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>,
) -> Result<(axum::http::HeaderMap, Json<crate::models::tap_time::TapTimePinsResponse>), AppError> {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    Ok((headers, Json(service.pins(&auth, query.school_id).await?)))
}

pub async fn get_tap_time_reconciliation(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(school_id): Path<Uuid>,
) -> Result<Json<Vec<ReconciliationProposal>>, AppError> { Ok(Json(service.reconciliation_proposals(&auth, school_id).await?)) }

pub async fn get_tap_time_dashboard(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(school_id): Path<Uuid>,
) -> Result<Json<TapTimeIntegrationDashboard>, AppError> { Ok(Json(service.integration_dashboard(&auth, school_id).await?)) }

pub async fn confirm_tap_time_reconciliation(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(school_id): Path<Uuid>, Json(request): Json<ConfirmReconciliationRequest>,
) -> Result<StatusCode, AppError> { service.confirm_reconciliation(&auth, school_id, request).await?; Ok(StatusCode::NO_CONTENT) }

pub async fn list_time_reports(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<TimeAttendanceQuery>,
) -> Result<Json<Vec<TapTimeReport>>, AppError> { Ok(Json(service.list_reports(&auth, query).await?)) }

pub async fn list_time_report_people(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>,
) -> Result<Json<Vec<TapTimeReportPerson>>, AppError> { Ok(Json(service.list_report_people(&auth, query.school_id).await?)) }

pub async fn create_time_report(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>, Json(request): Json<CreateTimeReportRequest>,
) -> Result<(StatusCode, Json<TapTimeReport>), AppError> { Ok((StatusCode::CREATED, Json(service.create_report(&auth, query.school_id, request).await?))) }

pub async fn get_time_report_overview(State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<TimeAttendanceSummaryQuery>) -> Result<Json<crate::models::tap_time::TapTimeReportOverview>, AppError> { Ok(Json(service.report_overview(&auth, query.school_id, query.report_date).await?)) }

pub async fn get_time_report_two_day(State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<TimeAttendanceSummaryQuery>) -> Result<Json<crate::models::tap_time::TapTimeTwoDayReport>, AppError> { Ok(Json(service.two_day_report(&auth, query.school_id, query.report_date).await?)) }

pub async fn get_time_report_salary(State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<TimeAttendanceSummaryQuery>) -> Result<Json<Vec<crate::models::tap_time::TapTimeSalaryPeriod>>, AppError> { Ok(Json(service.salary_report(&auth, query.school_id, query.anchor_date).await?)) }

pub async fn get_consolidated_time_report(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<TimeAttendanceQuery>,
) -> Result<Json<Vec<TapTimeConsolidatedReport>>, AppError> {
    let start_date = query.start_date.ok_or_else(|| AppError::Validation("start_date is required".to_string()))?;
    let end_date = query.end_date.ok_or_else(|| AppError::Validation("end_date is required".to_string()))?;
    Ok(Json(service.consolidated_report(&auth, query.school_id, start_date, end_date).await?))
}

pub async fn get_time_report_day_trends(State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<TimeAttendanceSummaryQuery>) -> Result<Json<Vec<crate::models::tap_time::TapTimeDayTrend>>, AppError> { Ok(Json(service.day_trends(&auth, query.school_id, query.start_date, query.end_date).await?)) }

pub async fn get_my_daily_time_reports(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<MyDailyQuery>,
) -> Result<Json<Vec<TapTimeReport>>, AppError> { Ok(Json(service.my_daily_reports(&auth, query.report_date).await?)) }

#[derive(Deserialize)]
pub struct MyDailyQuery { pub report_date: Option<chrono::NaiveDate> }

#[derive(Deserialize)]
pub struct TimeReportUpdateQuery { pub school_id: Uuid }

#[derive(Deserialize)]
pub struct TimeReportDeleteQuery { pub school_id: Uuid, pub reason: String }

pub async fn update_time_report(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(report_id): Path<Uuid>, Query(query): Query<TimeReportUpdateQuery>, Json(request): Json<UpdateTimeReportRequest>,
) -> Result<Json<TapTimeReport>, AppError> { Ok(Json(service.update_report(&auth, query.school_id, report_id, request).await?)) }

pub async fn delete_time_report(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(report_id): Path<Uuid>, Query(query): Query<TimeReportDeleteQuery>,
) -> Result<StatusCode, AppError> { service.delete_report(&auth, query.school_id, report_id, query.reason).await?; Ok(StatusCode::NO_CONTENT) }

pub async fn list_time_report_settings(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>,
) -> Result<Json<Vec<TapTimeReportSetting>>, AppError> { Ok(Json(service.list_report_settings(&auth, query.school_id).await?)) }

pub async fn create_time_report_setting(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>, Json(request): Json<UpsertTimeReportSettingRequest>,
) -> Result<(StatusCode, Json<TapTimeReportSetting>), AppError> { Ok((StatusCode::CREATED, Json(service.create_report_setting(&auth, query.school_id, request).await?))) }

pub async fn update_time_report_setting(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(setting_id): Path<Uuid>, Query(query): Query<SchoolQuery>, Json(request): Json<UpsertTimeReportSettingRequest>,
) -> Result<Json<TapTimeReportSetting>, AppError> { Ok(Json(service.update_report_setting(&auth, query.school_id, setting_id, request).await?)) }

pub async fn delete_time_report_setting(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Path(setting_id): Path<Uuid>, Query(query): Query<SchoolQuery>,
) -> Result<StatusCode, AppError> { service.delete_report_setting(&auth, query.school_id, setting_id).await?; Ok(StatusCode::NO_CONTENT) }

pub async fn get_consolidated_time_report_setting(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>,
) -> Result<Json<TapTimeConsolidatedReportSetting>, AppError> { Ok(Json(service.consolidated_report_setting(&auth, query.school_id).await?)) }

pub async fn update_consolidated_time_report_setting(
    State(service): State<Arc<TapTimeService>>, axum::Extension(auth): axum::Extension<AuthContext>, Query(query): Query<SchoolQuery>, Json(request): Json<UpdateTapTimeConsolidatedReportSettingRequest>,
) -> Result<Json<TapTimeConsolidatedReportSetting>, AppError> { Ok(Json(service.update_consolidated_report_setting(&auth, query.school_id, request).await?)) }
