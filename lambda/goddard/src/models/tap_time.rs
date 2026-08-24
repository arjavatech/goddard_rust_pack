use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The one-time value is accepted only to redeem a connection and is never persisted.
#[derive(Debug, Deserialize)]
pub struct ConnectTapTimeRequest {
    pub school_id: Uuid,
    pub connection_code: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TapTimeConnection {
    pub school_id: Uuid,
    pub tap_company_id: Uuid,
    pub tap_company_name: String,
    pub tap_timezone: Option<String>,
    pub status: String,
    pub connected_by: Uuid,
    pub connected_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub last_health_check_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TapTimeRemoteConnection {
    pub connection_id: Uuid,
    pub company_id: Uuid,
    pub company_name: Option<String>,
    pub timezone: Option<String>,
    pub status: String,
    pub access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct TapTimeRemoteEmployee {
    pub internal_employee_id: Uuid,
    pub sync_status: String,
}

#[derive(Debug, Clone)]
pub struct TapTimeSyncJob {
    pub id: Uuid,
    pub school_id: Uuid,
    pub employee_id: Uuid,
    pub operation: String,
}

#[derive(Debug, Clone)]
pub struct EmployeeSyncData {
    pub id: Uuid,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: String,
    pub is_active: bool,
    /// Tap-Time access level: 0 employee, 1 admin, 2 super admin.
    pub is_admin: i32,
}

#[derive(Debug, Deserialize)]
pub struct SetTapTimePinRequest {
    pub school_id: Uuid,
    pub pin: String,
}

#[derive(Debug, Deserialize)]
pub struct SetMyTapTimePinRequest { pub pin: String }

#[derive(Debug, Deserialize)]
pub struct SetTapTimeUserPinRequest {
    pub school_id: Uuid,
    pub pin: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimePinResponse {
    pub pin: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TapTimeRemotePinsResponse {
    pub pins: Vec<TapTimeRemotePin>,
}

#[derive(Debug, Deserialize)]
pub struct TapTimeRemotePin {
    pub external_entity_id: Uuid,
    pub pin: String,
}

#[derive(Debug, Serialize)]
pub struct TapTimePinsResponse {
    pub pins: HashMap<Uuid, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TapTimeEmployeeCandidate {
    pub internal_employee_id: Uuid,
    pub external_employee_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
    pub email: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationProposal {
    pub employee_id: Uuid,
    pub employee_name: String,
    pub entity_type: String,
    pub role: String,
    pub match_type: String,
    pub match_value: String,
    pub tap_employee_id: Uuid,
    pub tap_employee_name: String,
    pub normalized_phone: String,
}

#[derive(Debug, Serialize)]
pub struct TapTimeRoleLinkSummary {
    pub total: i64,
    pub linked: i64,
}

#[derive(Debug, Serialize)]
pub struct TapTimeIntegrationOverview {
    pub tap_time_people_total: i64,
    pub employees: TapTimeRoleLinkSummary,
    pub admins: TapTimeRoleLinkSummary,
    pub super_admins: TapTimeRoleLinkSummary,
    pub needs_review: i64,
    pub failed_syncs: i64,
}

#[derive(Debug, Serialize)]
pub struct TapTimeLinkedPerson {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub role: String,
    pub person_name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub tap_employee_id: Uuid,
    pub tap_employee_name: Option<String>,
    pub sync_status: String,
    pub linked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct TapTimeIntegrationDashboard {
    pub overview: TapTimeIntegrationOverview,
    pub linked_people: Vec<TapTimeLinkedPerson>,
    pub suggestions: Vec<ReconciliationProposal>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmReconciliationRequest { pub employee_id: Uuid, pub tap_employee_id: Uuid, pub entity_type: String }

#[derive(Debug, Serialize)]
pub struct SyncOutcome {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub entity_name: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeReport {
    pub report_id: Uuid,
    pub external_employee_id: Option<Uuid>,
    pub internal_employee_id: Uuid,
    pub employee_name: String,
    pub date: Option<chrono::NaiveDate>,
    // Tap-Time stores local wall-clock timestamps without an offset.  Keeping
    // these naive avoids rejecting its valid JSON as a UTC-only timestamp.
    pub check_in_time: NaiveDateTime,
    pub check_out_time: Option<NaiveDateTime>,
    pub time_worked: Option<String>,
    pub type_id: Option<String>,
    pub pending_checkout: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeConsolidatedReport {
    pub external_employee_id: Uuid,
    pub internal_employee_id: Uuid,
    pub employee_name: String,
    pub worked_minutes: i64,
    pub total_time_worked: String,
}

#[derive(Debug, Deserialize)]
pub struct TimeAttendanceQuery {
    pub school_id: Uuid,
    pub report_date: Option<chrono::NaiveDate>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub pending_checkout: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTimeReportRequest {
    pub check_in_time: Option<NaiveDateTime>,
    pub check_out_time: Option<NaiveDateTime>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTimeReportRequest {
    pub external_employee_id: Uuid,
    pub report_date: NaiveDate,
    pub check_in_time: NaiveDateTime,
    pub check_out_time: Option<NaiveDateTime>,
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeReportPerson {
    pub external_employee_id: Uuid,
    pub internal_employee_id: Uuid,
    pub employee_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeReportSetting {
    pub setting_id: Uuid,
    pub reporter_email: String,
    pub is_daily_report_active: bool,
    pub is_weekly_report_active: bool,
    pub is_bi_weekly_report_active: bool,
    pub is_monthly_report_active: bool,
    pub is_bi_monthly_report_active: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpsertTimeReportSettingRequest {
    pub reporter_email: String,
    pub is_daily_report_active: bool,
    pub is_weekly_report_active: bool,
    pub is_bi_weekly_report_active: bool,
    pub is_monthly_report_active: bool,
    pub is_bi_monthly_report_active: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeConsolidatedReportSetting {
    pub report_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTapTimeConsolidatedReportSettingRequest {
    pub report_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeReportOverview {
    pub report_date: chrono::NaiveDate,
    pub employee_count: i32,
    pub record_count: i32,
    pub completed_count: i32,
    pub pending_checkout_count: i32,
    pub worked_minutes: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeSalaryPeriod {
    pub label: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub employee_count: i32,
    pub worked_minutes: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeDayTrend {
    pub report_date: chrono::NaiveDate,
    pub employee_count: i32,
    pub record_count: i32,
    pub completed_count: i32,
    pub pending_checkout_count: i32,
    pub worked_minutes: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TapTimeTwoDayReport {
    pub current: TapTimeReportOverview,
    pub previous: TapTimeReportOverview,
}

#[derive(Debug, Deserialize)]
pub struct TimeAttendanceSummaryQuery {
    pub school_id: Uuid,
    pub report_date: Option<chrono::NaiveDate>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub anchor_date: Option<chrono::NaiveDate>,
}
