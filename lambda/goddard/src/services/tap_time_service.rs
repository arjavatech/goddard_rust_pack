use std::env;

use aes_gcm::{aead::{Aead, AeadCore, KeyInit, OsRng}, Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures::{future::BoxFuture, stream, FutureExt, StreamExt};
use reqwest::Client;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    dao::TapTimeDao,
    error::error_types::AppError,
    models::{schema::UserRole, tap_time::{ConfirmReconciliationRequest, ConnectTapTimeRequest, CreateTimeReportRequest, EmployeeSyncData, ReconciliationProposal, SetMyTapTimePinRequest, SetTapTimePinRequest, SetTapTimeUserPinRequest, SyncOutcome, TapTimeConnection, TapTimeConsolidatedReport, TapTimeConsolidatedReportSetting, TapTimeDayTrend, TapTimeEmployeeCandidate, TapTimeIntegrationDashboard, TapTimeIntegrationOverview, TapTimePinResponse, TapTimePinsResponse, TapTimeRemoteConnection, TapTimeRemoteEmployee, TapTimeRemotePinsResponse, TapTimeReport, TapTimeReportOverview, TapTimeReportPerson, TapTimeReportSetting, TapTimeSalaryPeriod, TapTimeTwoDayReport, TimeAttendanceQuery, UpdateTapTimeConsolidatedReportSettingRequest, UpdateTimeReportRequest, UpsertTimeReportSettingRequest}},
    middleware::auth::AuthContext,
};

#[derive(Clone)]
pub struct TapTimeService {
    dao: TapTimeDao,
    http: Client,
    config: Option<TapTimeIntegrationConfig>,
}

#[derive(Clone)]
struct TapTimeIntegrationConfig {
    base_url: String,
    encryption_key: [u8; 32],
}

#[derive(Serialize)]
struct RedeemConnectionRequest<'a> {
    external_tenant_id: Uuid,
    connection_code: &'a str,
    external_actor_id: Uuid,
}

#[derive(Serialize)]
struct EmployeeUpsertRequest<'a> {
    first_name: &'a str, last_name: &'a str, phone_number: Option<&'a str>, email: Option<&'a str>, is_active: bool, is_admin: i32, external_actor_id: Uuid,
}

#[derive(Serialize)]
struct PinUpdateRequest<'a> { pin: &'a str, external_actor_id: Uuid }

#[derive(Serialize)]
struct ReconcileEmployeeRequest { external_employee_id: Uuid, internal_employee_id: Uuid, external_actor_id: Uuid }

#[derive(Serialize)]
struct RemoteReportUpdate<'a> { check_in_time: Option<chrono::NaiveDateTime>, check_out_time: Option<chrono::NaiveDateTime>, reason: &'a str, external_actor_id: Uuid }

#[derive(Serialize)]
struct RemoteReportCreate<'a> { external_employee_id: Uuid, check_in_time: chrono::NaiveDateTime, check_out_time: Option<chrono::NaiveDateTime>, reason: &'a str, external_actor_id: Uuid }

#[derive(Serialize)]
struct RemoteReportSettingUpsert<'a> {
    reporter_email: &'a str,
    is_daily_report_active: bool,
    is_weekly_report_active: bool,
    is_bi_weekly_report_active: bool,
    is_monthly_report_active: bool,
    is_bi_monthly_report_active: bool,
    external_actor_id: Uuid,
}

#[derive(Serialize)]
pub struct SyncDispatchResponse { pub processed: usize, pub succeeded: usize, pub failed: usize, pub outcomes: Vec<SyncOutcome> }

impl TapTimeService {
    pub fn from_env(dao: TapTimeDao) -> Self {
        let config = Self::load_config();
        match &config {
            Ok(_) => println!("[TapTime] one-code integration configured"),
            Err(_) => println!("[TapTime] integration disabled; TAP_TIME_API_URL and TAP_TIME_CONNECTION_ENCRYPTION_KEY are required"),
        }
        let http = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Tap-Time HTTP client configuration is valid");
        Self { dao, http, config: config.ok() }
    }

    fn load_config() -> Result<TapTimeIntegrationConfig, AppError> {
        (|| -> Result<TapTimeIntegrationConfig, AppError> {
            let required = |name: &str| env::var(name).map_err(|_| AppError::Internal(format!("{name} must be configured before enabling Tap-Time integration")));
            Self::validated_config(required("TAP_TIME_API_URL")?, required("TAP_TIME_CONNECTION_ENCRYPTION_KEY")?)
        })()
    }

    fn validated_config(base_url: String, encryption_key: String) -> Result<TapTimeIntegrationConfig, AppError> {
        let base_url = base_url.trim_end_matches('/').to_string();
        if !(base_url.starts_with("https://") || base_url.starts_with("http://localhost") || base_url.starts_with("http://127.0.0.1")) {
            return Err(AppError::Internal("Tap-Time API URL must use HTTPS outside local development".to_string()));
        }
        let decoded = STANDARD.decode(encryption_key.trim()).map_err(|_| AppError::Internal("TAP_TIME_CONNECTION_ENCRYPTION_KEY must be base64 encoded".to_string()))?;
        let key: [u8; 32] = decoded.try_into().map_err(|_| AppError::Internal("TAP_TIME_CONNECTION_ENCRYPTION_KEY must decode to exactly 32 bytes".to_string()))?;
        Ok(TapTimeIntegrationConfig { base_url, encryption_key: key })
    }

    pub async fn get_connection(&self, auth: &AuthContext, school_id: Uuid) -> Result<Option<TapTimeConnection>, AppError> {
        self.require_superadmin(auth)?;
        self.dao.get_connection(school_id).await
    }

    pub async fn connect(&self, auth: &AuthContext, request: ConnectTapTimeRequest) -> Result<TapTimeConnection, AppError> {
        self.require_superadmin(auth)?;
        if request.connection_code.trim().len() < 16 {
            return Err(AppError::Validation("connection_code is invalid".to_string()));
        }
        let remote = self.redeem_connection_code(request.school_id, request.connection_code.trim(), auth.user_id).await?;
        if remote.status != "active" {
            return Err(AppError::ExternalService("Tap-Time did not activate the connection".to_string()));
        }
        let (ciphertext, nonce) = self.encrypt_connection_token(&remote.access_token)?;
        self.dao.save_connection(
            request.school_id,
            remote.company_id,
            remote.company_name.as_deref().unwrap_or("Connected company"),
            remote.timezone.as_deref(),
            auth.user_id, &ciphertext, &nonce,
        ).await
    }

    pub async fn disconnect(&self, auth: &AuthContext, school_id: Uuid) -> Result<TapTimeConnection, AppError> {
        self.require_superadmin(auth)?;
        let existing = self.dao.get_connection(school_id).await?
            .ok_or_else(|| AppError::NotFound("Tap-Time connection".to_string()))?;
        if existing.status == "active" {
            self.delete_remote_connection(school_id).await?;
        }
        self.dao.mark_disconnected(school_id, auth.user_id).await
    }

    pub async fn dispatch_employee_sync(&self, auth: &AuthContext, school_id: Uuid) -> Result<SyncDispatchResponse, AppError> {
        self.require_superadmin(auth)?;
        let connection = self.dao.get_connection(school_id).await?.ok_or_else(|| AppError::NotFound("Tap-Time connection".to_string()))?;
        if connection.status != "active" { return Err(AppError::Conflict("Tap-Time connection is not active".to_string())); }
        if !self.dao.acquire_sync_lock(school_id).await? {
            return Err(AppError::Conflict("A Tap-Time sync is already in progress for this school. Please wait for it to finish.".to_string()));
        }
        let result = self.dispatch_employee_sync_locked(auth.user_id, school_id).await;
        if let Err(error) = self.dao.release_sync_lock(school_id).await {
            eprintln!("[TapTime] failed to release sync lock for {school_id}: {error}");
        }
        result
    }

    async fn dispatch_employee_sync_locked(&self, actor: Uuid, school_id: Uuid) -> Result<SyncDispatchResponse, AppError> {
        let jobs = self.dao.claim_sync_jobs(school_id, 25).await?;
        let admins = self.dao.school_admin_sync_data(school_id).await?;
        let processed = jobs.len() + admins.len();
        let mut tasks: Vec<BoxFuture<'static, SyncOutcome>> = Vec::with_capacity(processed);
        for admin in admins {
            let service = self.clone();
            tasks.push(async move { service.sync_admin(admin, actor).await }.boxed());
        }
        for job in jobs {
            let service = self.clone();
            tasks.push(async move { service.sync_employee_job(job, actor).await }.boxed());
        }
        let outcomes = stream::iter(tasks).buffer_unordered(8).collect::<Vec<_>>().await;
        let succeeded = outcomes.iter().filter(|value| value.status == "synced").count();
        let failed = outcomes.len() - succeeded;
        Ok(SyncDispatchResponse { processed, succeeded, failed, outcomes })
    }

    async fn sync_admin(&self, admin: EmployeeSyncData, actor: Uuid) -> SyncOutcome {
        let entity_name = format!("{} {}", admin.first_name, admin.last_name);
        let entity_type = if admin.is_admin == 2 { "super_admin" } else { "admin" }.to_string();
        let outcome = async {
            let remote = self.upsert_remote_employee(&admin, actor).await?;
            self.dao.complete_admin_sync(admin.school_id, admin.id, remote.internal_employee_id, &remote.sync_status).await
        }.await;
        match outcome {
            Ok(()) => SyncOutcome { entity_id: admin.id, entity_type, entity_name, status: "synced".to_string(), error: None },
            Err(error) => {
                let message = error.to_string();
                if let Err(record_error) = self.dao.fail_admin_sync(admin.school_id, admin.id, &message).await { eprintln!("[TapTime] failed to save admin sync failure: {record_error}"); }
                SyncOutcome { entity_id: admin.id, entity_type, entity_name, status: "failed".to_string(), error: Some(message) }
            }
        }
    }

    async fn sync_employee_job(&self, job: crate::models::tap_time::TapTimeSyncJob, actor: Uuid) -> SyncOutcome {
        let entity_id = job.employee_id;
        let outcome = async {
            let data = self.dao.employee_sync_data(job.employee_id, job.school_id).await?;
            let entity_name = format!("{} {}", data.first_name, data.last_name);
            let remote = self.upsert_remote_employee(&data, actor).await?;
            self.dao.complete_sync_job(&job, remote.internal_employee_id, &remote.sync_status).await?;
            Ok::<String, AppError>(entity_name)
        }.await;
        match outcome {
            Ok(entity_name) => SyncOutcome { entity_id, entity_type: "employee".to_string(), entity_name, status: "synced".to_string(), error: None },
            Err(error) => {
                let message = error.to_string();
                if let Err(record_error) = self.dao.fail_sync_job(job.id, &message).await { eprintln!("[TapTime] failed to save employee sync failure: {record_error}"); }
                SyncOutcome { entity_id, entity_type: "employee".to_string(), entity_name: entity_id.to_string(), status: "failed".to_string(), error: Some(message) }
            }
        }
    }

    pub async fn set_employee_pin(&self, auth: &AuthContext, employee_id: Uuid, request: SetTapTimePinRequest) -> Result<(), AppError> {
        match auth.role { UserRole::Admin if auth.school_id == request.school_id => {}, UserRole::SuperAdmin => {}, _ => return Err(AppError::Authorization("Only an administrator can reset another employee's Tap-Time PIN".to_string())) }
        self.validate_pin(&request.pin)?;
        self.dao.ensure_linked_employee(employee_id, request.school_id).await?;
        self.update_remote_pin(request.school_id, employee_id, &request.pin, auth.user_id).await?;
        self.dao.record_audit(request.school_id, auth.user_id, "employee_pin_reset", employee_id).await
    }

    pub async fn employee_pin(&self, auth: &AuthContext, employee_id: Uuid, school_id: Uuid) -> Result<TapTimePinResponse, AppError> {
        match auth.role { UserRole::Admin if auth.school_id == school_id => {}, UserRole::SuperAdmin if auth.school_id == school_id => {}, _ => return Err(AppError::Authorization("Only an administrator can view an employee's Tap-Time PIN".to_string())) }
        self.dao.ensure_linked_employee(employee_id, school_id).await?;
        self.remote_pin(school_id, employee_id).await
    }

    pub async fn set_my_pin(&self, auth: &AuthContext, request: SetMyTapTimePinRequest) -> Result<(), AppError> {
        self.validate_pin(&request.pin)?;
        let external_id = match self.dao.linked_employee_for_user(auth.user_id, auth.school_id).await {
            Ok(employee_id) => employee_id,
            Err(AppError::NotFound(_)) => { self.dao.ensure_linked_user(auth.user_id, auth.school_id).await?; auth.user_id },
            Err(error) => return Err(error),
        };
        self.update_remote_pin(auth.school_id, external_id, &request.pin, auth.user_id).await?;
        self.dao.record_user_audit(auth.school_id, auth.user_id, "tap_time_pin_reset_self", external_id).await
    }

    pub async fn my_pin(&self, auth: &AuthContext) -> Result<TapTimePinResponse, AppError> {
        let external_id = match self.dao.linked_employee_for_user(auth.user_id, auth.school_id).await {
            Ok(employee_id) => employee_id,
            Err(AppError::NotFound(_)) => { self.dao.ensure_linked_user(auth.user_id, auth.school_id).await?; auth.user_id },
            Err(error) => return Err(error),
        };
        self.remote_pin(auth.school_id, external_id).await
    }

    pub async fn set_admin_pin(&self, auth: &AuthContext, user_id: Uuid, request: SetTapTimeUserPinRequest) -> Result<(), AppError> {
        if !matches!(auth.role, UserRole::Admin | UserRole::SuperAdmin) || auth.school_id != request.school_id {
            return Err(AppError::Authorization("Only this school's administrator can reset an administrator's Tap-Time PIN".to_string()));
        }
        self.validate_pin(&request.pin)?;
        self.dao.ensure_linked_user(user_id, request.school_id).await?;
        self.update_remote_pin(request.school_id, user_id, &request.pin, auth.user_id).await?;
        self.dao.record_user_audit(request.school_id, auth.user_id, "admin_pin_reset", user_id).await
    }

    pub async fn admin_pin(&self, auth: &AuthContext, user_id: Uuid, school_id: Uuid) -> Result<TapTimePinResponse, AppError> {
        if !matches!(auth.role, UserRole::Admin | UserRole::SuperAdmin) || auth.school_id != school_id {
            return Err(AppError::Authorization("Only this school's administrator can view an administrator's Tap-Time PIN".to_string()));
        }
        self.dao.ensure_linked_user(user_id, school_id).await?;
        self.remote_pin(school_id, user_id).await
    }

    pub async fn pins(&self, auth: &AuthContext, school_id: Uuid) -> Result<TapTimePinsResponse, AppError> {
        self.require_school_admin(auth, school_id)?;
        let connection = self.dao.get_connection(school_id).await?;
        if !matches!(connection.as_ref().map(|value| value.status.as_str()), Some("active")) {
            return Ok(TapTimePinsResponse { pins: std::collections::HashMap::new() });
        }
        match self.remote_pins(school_id).await {
            Ok(remote) => Ok(TapTimePinsResponse { pins: remote.pins.into_iter().map(|value| (value.external_entity_id, value.pin)).collect() }),
            Err(AppError::NotFound(_)) => Ok(TapTimePinsResponse { pins: std::collections::HashMap::new() }),
            Err(error) => Err(error),
        }
    }

    pub async fn reconciliation_proposals(&self, auth: &AuthContext, school_id: Uuid) -> Result<Vec<ReconciliationProposal>, AppError> {
        self.require_superadmin(auth)?;
        let remote = self.remote_employee_candidates(school_id).await?;
        self.reconciliation_proposals_for_remote(school_id, remote).await
    }

    pub async fn integration_dashboard(&self, auth: &AuthContext, school_id: Uuid) -> Result<TapTimeIntegrationDashboard, AppError> {
        self.require_superadmin(auth)?;
        let connection = self.dao.get_connection(school_id).await?
            .ok_or_else(|| AppError::NotFound("Tap-Time connection".to_string()))?;
        if connection.status != "active" { return Err(AppError::Conflict("Tap-Time connection is not active".to_string())); }
        let remote = self.remote_employee_candidates(school_id).await?;
        let (employees, admins, super_admins, failed_syncs) = self.dao.integration_role_summaries(school_id).await?;
        let mut linked_people = self.dao.linked_people(school_id).await?;
        let remote_names: std::collections::HashMap<Uuid, String> = remote.iter()
            .map(|person| (person.internal_employee_id, format!("{} {}", person.first_name, person.last_name).trim().to_string()))
            .collect();
        for person in &mut linked_people { person.tap_employee_name = remote_names.get(&person.tap_employee_id).cloned(); }
        let suggestions = self.reconciliation_proposals_for_remote(school_id, remote.clone()).await?;
        Ok(TapTimeIntegrationDashboard {
            overview: TapTimeIntegrationOverview { tap_time_people_total: remote.len() as i64, employees, admins, super_admins, needs_review: suggestions.len() as i64, failed_syncs },
            linked_people,
            suggestions,
        })
    }

    async fn reconciliation_proposals_for_remote(&self, school_id: Uuid, remote: Vec<TapTimeEmployeeCandidate>) -> Result<Vec<ReconciliationProposal>, AppError> {
        let mut local = self.dao.school_employee_sync_data(school_id).await?;
        local.extend(self.dao.school_admin_sync_data(school_id).await?);
        let mut local_by_phone: std::collections::HashMap<String, Vec<EmployeeSyncData>> = std::collections::HashMap::new();
        let mut local_by_email: std::collections::HashMap<String, Vec<EmployeeSyncData>> = std::collections::HashMap::new();
        let mut remote_by_phone: std::collections::HashMap<String, Vec<TapTimeEmployeeCandidate>> = std::collections::HashMap::new();
        let mut remote_by_email: std::collections::HashMap<String, Vec<TapTimeEmployeeCandidate>> = std::collections::HashMap::new();
        for employee in local {
            if let Some(phone) = employee.phone.as_deref().and_then(normalize_phone) { local_by_phone.entry(phone).or_default().push(employee.clone()); }
            if let Some(email) = normalize_email(&employee.email) { local_by_email.entry(email).or_default().push(employee); }
        }
        for employee in remote {
            if employee.external_employee_id.is_none() {
                if let Some(phone) = employee.phone_number.as_deref().and_then(normalize_phone) { remote_by_phone.entry(phone).or_default().push(employee.clone()); }
                if let Some(email) = employee.email.as_deref().and_then(normalize_email) { remote_by_email.entry(email).or_default().push(employee); }
            }
        }
        let mut proposals = Vec::new();
        let mut used_local = std::collections::HashSet::new();
        let mut used_remote = std::collections::HashSet::new();
        for (phone, employees) in local_by_phone {
            if employees.len() != 1 { continue; }
            let Some(candidates) = remote_by_phone.get(&phone) else { continue; };
            if candidates.len() != 1 { continue; }
            let employee = &employees[0]; let candidate = &candidates[0];
            used_local.insert(employee.id); used_remote.insert(candidate.internal_employee_id);
            proposals.push(reconciliation_proposal(employee, candidate, "phone", &phone));
        }
        for (email, employees) in local_by_email {
            if employees.len() != 1 { continue; }
            let Some(candidates) = remote_by_email.get(&email) else { continue; };
            if candidates.len() != 1 { continue; }
            let employee = &employees[0]; let candidate = &candidates[0];
            if used_local.contains(&employee.id) || used_remote.contains(&candidate.internal_employee_id) { continue; }
            used_local.insert(employee.id); used_remote.insert(candidate.internal_employee_id);
            proposals.push(reconciliation_proposal(employee, candidate, "email", &email));
        }
        Ok(proposals)
    }

    pub async fn confirm_reconciliation(&self, auth: &AuthContext, school_id: Uuid, request: ConfirmReconciliationRequest) -> Result<(), AppError> {
        self.require_superadmin(auth)?;
        if request.entity_type != "employee" && request.entity_type != "user" {
            return Err(AppError::Validation("entity_type must be employee or user".to_string()));
        }
        let proposal_is_current = self.reconciliation_proposals(auth, school_id).await?.iter().any(|proposal| {
            proposal.employee_id == request.employee_id
                && proposal.tap_employee_id == request.tap_employee_id
                && proposal.entity_type == request.entity_type
        });
        if !proposal_is_current {
            return Err(AppError::Validation("This Tap-Time match is no longer an unambiguous, unlinked candidate. Refresh and review the available matches.".to_string()));
        }
        self.reconcile_remote_employee(school_id, request.employee_id, request.tap_employee_id, auth.user_id).await?;
        match request.entity_type.as_str() {
            "employee" => self.dao.save_reconciliation(school_id, request.employee_id, request.tap_employee_id, auth.user_id).await,
            "user" => self.dao.save_user_reconciliation(school_id, request.employee_id, request.tap_employee_id, auth.user_id).await,
            _ => unreachable!("entity_type is validated before remote reconciliation"),
        }
    }

    pub async fn list_reports(&self, auth: &AuthContext, query: TimeAttendanceQuery) -> Result<Vec<TapTimeReport>, AppError> {
        match auth.role { UserRole::SuperAdmin => {}, UserRole::Admin if auth.school_id == query.school_id => {}, _ => return Err(AppError::Authorization("Only an administrator can view school time reports".to_string())) }
        self.remote_reports(query.school_id, query.report_date, query.start_date, query.end_date, None, query.pending_checkout).await
    }

    pub async fn list_report_people(&self, auth: &AuthContext, school_id: Uuid) -> Result<Vec<TapTimeReportPerson>, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_report_people(school_id).await
    }

    pub async fn create_report(&self, auth: &AuthContext, school_id: Uuid, request: CreateTimeReportRequest) -> Result<TapTimeReport, AppError> {
        self.require_school_admin(auth, school_id)?;
        if request.reason.trim().len() < 3 { return Err(AppError::Validation("A report creation reason is required".to_string())); }
        if request.check_in_time.date() != request.report_date { return Err(AppError::Validation("Check-in time must use the selected report date".to_string())); }
        if let Some(check_out) = request.check_out_time {
            if check_out.date() != request.report_date || check_out - request.check_in_time < chrono::Duration::minutes(1) { return Err(AppError::Validation("Check-out time must use the report date and be at least one minute after check-in".to_string())); }
        }
        let report = self.remote_create_report(school_id, &request, auth.user_id).await?;
        self.dao.record_audit(school_id, auth.user_id, "time_report_created", report.report_id).await?;
        Ok(report)
    }

    pub async fn report_overview(&self, auth: &AuthContext, school_id: Uuid, report_date: Option<chrono::NaiveDate>) -> Result<TapTimeReportOverview, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_summary(school_id, "overview", report_date).await
    }

    pub async fn two_day_report(&self, auth: &AuthContext, school_id: Uuid, report_date: Option<chrono::NaiveDate>) -> Result<TapTimeTwoDayReport, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_two_day_summary(school_id, report_date).await
    }

    pub async fn salary_report(&self, auth: &AuthContext, school_id: Uuid, anchor_date: Option<chrono::NaiveDate>) -> Result<Vec<TapTimeSalaryPeriod>, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_salary_summary(school_id, anchor_date).await
    }

    pub async fn consolidated_report(&self, auth: &AuthContext, school_id: Uuid, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> Result<Vec<TapTimeConsolidatedReport>, AppError> {
        self.require_school_admin(auth, school_id)?;
        if start_date > end_date { return Err(AppError::Validation("start_date must not be after end_date".to_string())); }
        self.remote_consolidated_report(school_id, start_date, end_date).await
    }

    pub async fn day_trends(&self, auth: &AuthContext, school_id: Uuid, start_date: Option<chrono::NaiveDate>, end_date: Option<chrono::NaiveDate>) -> Result<Vec<TapTimeDayTrend>, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_day_trends(school_id, start_date, end_date).await
    }

    pub async fn my_daily_reports(&self, auth: &AuthContext, report_date: Option<chrono::NaiveDate>) -> Result<Vec<TapTimeReport>, AppError> {
        let employee_id = self.dao.linked_employee_for_user(auth.user_id, auth.school_id).await?;
        self.remote_reports(auth.school_id, report_date, None, None, Some(employee_id), None).await
    }

    pub async fn update_report(&self, auth: &AuthContext, school_id: Uuid, report_id: Uuid, request: UpdateTimeReportRequest) -> Result<TapTimeReport, AppError> {
        match auth.role { UserRole::SuperAdmin => {}, UserRole::Admin if auth.school_id == school_id => {}, _ => return Err(AppError::Authorization("Only an administrator can update school time reports".to_string())) }
        if request.reason.trim().len() < 3 { return Err(AppError::Validation("A report update reason is required".to_string())); }
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        let report = self.http.patch(format!("{}/integrations/v1/tenants/{}/reports/{}", base_url, school_id, report_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .json(&RemoteReportUpdate { check_in_time: request.check_in_time, check_out_time: request.check_out_time, reason: request.reason.trim(), external_actor_id: auth.user_id })
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report update request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report update: {e}")))?
            .json::<TapTimeReport>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time report update response: {e}")))?;
        self.dao.record_audit(school_id, auth.user_id, "time_report_updated", report_id).await?;
        Ok(report)
    }

    pub async fn delete_report(&self, auth: &AuthContext, school_id: Uuid, report_id: Uuid, reason: String) -> Result<(), AppError> {
        match auth.role { UserRole::SuperAdmin => {}, UserRole::Admin if auth.school_id == school_id => {}, _ => return Err(AppError::Authorization("Only an administrator can delete school time reports".to_string())) }
        if reason.trim().len() < 3 { return Err(AppError::Validation("A report delete reason is required".to_string())); }
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.delete(format!("{}/integrations/v1/tenants/{}/reports/{}", base_url, school_id, report_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string()).query(&[("reason", reason.trim())])
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report delete request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report delete: {e}")))?;
        self.dao.record_audit(school_id, auth.user_id, "time_report_deleted", report_id).await
    }

    pub async fn list_report_settings(&self, auth: &AuthContext, school_id: Uuid) -> Result<Vec<TapTimeReportSetting>, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_report_settings(school_id).await
    }

    pub async fn create_report_setting(&self, auth: &AuthContext, school_id: Uuid, request: UpsertTimeReportSettingRequest) -> Result<TapTimeReportSetting, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.validate_report_setting(&request)?;
        let setting = self.remote_create_report_setting(school_id, &request, auth.user_id).await?;
        self.dao.record_audit(school_id, auth.user_id, "time_report_setting_created", setting.setting_id).await?;
        Ok(setting)
    }

    pub async fn update_report_setting(&self, auth: &AuthContext, school_id: Uuid, setting_id: Uuid, request: UpsertTimeReportSettingRequest) -> Result<TapTimeReportSetting, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.validate_report_setting(&request)?;
        let setting = self.remote_update_report_setting(school_id, setting_id, &request, auth.user_id).await?;
        self.dao.record_audit(school_id, auth.user_id, "time_report_setting_updated", setting_id).await?;
        Ok(setting)
    }

    pub async fn delete_report_setting(&self, auth: &AuthContext, school_id: Uuid, setting_id: Uuid) -> Result<(), AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_delete_report_setting(school_id, setting_id).await?;
        self.dao.record_audit(school_id, auth.user_id, "time_report_setting_deleted", setting_id).await
    }

    pub async fn consolidated_report_setting(&self, auth: &AuthContext, school_id: Uuid) -> Result<TapTimeConsolidatedReportSetting, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.remote_consolidated_report_setting(school_id).await
    }

    pub async fn update_consolidated_report_setting(&self, auth: &AuthContext, school_id: Uuid, request: UpdateTapTimeConsolidatedReportSettingRequest) -> Result<TapTimeConsolidatedReportSetting, AppError> {
        self.require_school_admin(auth, school_id)?;
        self.validate_consolidated_report_type(&request.report_type)?;
        let setting = self.remote_update_consolidated_report_setting(school_id, &request.report_type, auth.user_id).await?;
        self.dao.record_user_audit(school_id, auth.user_id, "time_consolidated_report_setting_updated", auth.user_id).await?;
        Ok(setting)
    }

    fn require_superadmin(&self, auth: &AuthContext) -> Result<(), AppError> {
        if !matches!(auth.role, UserRole::SuperAdmin) {
            return Err(AppError::Authorization("Only Super Admin can manage a Tap-Time connection".to_string()));
        }
        Ok(())
    }

    fn require_school_admin(&self, auth: &AuthContext, school_id: Uuid) -> Result<(), AppError> {
        match auth.role {
            UserRole::SuperAdmin => Ok(()),
            UserRole::Admin if auth.school_id == school_id => Ok(()),
            _ => Err(AppError::Authorization("Only an administrator can manage school time-attendance settings".to_string())),
        }
    }

    fn validate_report_setting(&self, request: &UpsertTimeReportSettingRequest) -> Result<(), AppError> {
        if request.reporter_email.trim().is_empty() || !request.reporter_email.contains('@') {
            return Err(AppError::Validation("A valid report recipient email is required".to_string()));
        }
        if !(request.is_daily_report_active || request.is_weekly_report_active || request.is_bi_weekly_report_active || request.is_monthly_report_active || request.is_bi_monthly_report_active) {
            return Err(AppError::Validation("Enable at least one report schedule".to_string()));
        }
        let enabled = [request.is_daily_report_active, request.is_weekly_report_active, request.is_bi_weekly_report_active, request.is_monthly_report_active, request.is_bi_monthly_report_active].into_iter().filter(|value| *value).count();
        if enabled > 2 { return Err(AppError::Validation("Select no more than two report schedules".to_string())); }
        Ok(())
    }

    fn validate_consolidated_report_type(&self, report_type: &str) -> Result<(), AppError> {
        match report_type.trim() {
            "Daily" | "Weekly" | "Biweekly" | "Monthly" | "Bimonthly" => Ok(()),
            _ => Err(AppError::Validation("Select a valid consolidated report frequency".to_string())),
        }
    }

    fn validate_pin(&self, pin: &str) -> Result<(), AppError> {
        if pin.len() != 4 || !pin.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(AppError::Validation("PIN must contain exactly 4 digits".to_string()));
        }
        Ok(())
    }

    async fn connection_token(&self, tenant_id: Uuid) -> Result<String, AppError> {
        let config = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?;
        let (ciphertext, nonce) = self.dao.connection_access_token_material(tenant_id).await?
            .ok_or_else(|| AppError::Validation("Tap-Time connection credential is unavailable; reconnect Tap-Time".to_string()))?;
        let cipher = Aes256Gcm::new_from_slice(&config.encryption_key).map_err(|_| AppError::Internal("Invalid Tap-Time connection encryption key".to_string()))?;
        if nonce.len() != 12 { return Err(AppError::Internal("Stored Tap-Time connection credential is invalid".to_string())); }
        let value = cipher.decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| AppError::Internal("Unable to decrypt Tap-Time connection credential; reconnect Tap-Time".to_string()))?;
        String::from_utf8(value).map_err(|_| AppError::Internal("Stored Tap-Time connection credential is invalid".to_string()))
    }

    fn encrypt_connection_token(&self, token: &str) -> Result<(Vec<u8>, Vec<u8>), AppError> {
        let config = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?;
        let cipher = Aes256Gcm::new_from_slice(&config.encryption_key).map_err(|_| AppError::Internal("Invalid Tap-Time connection encryption key".to_string()))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, token.as_bytes()).map_err(|_| AppError::Internal("Unable to encrypt Tap-Time connection credential".to_string()))?;
        Ok((ciphertext, nonce.to_vec()))
    }

    async fn redeem_connection_code(&self, school_id: Uuid, code: &str, actor: Uuid) -> Result<TapTimeRemoteConnection, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4();
        self.http.post(format!("{}/integrations/v1/connections/redeem", base_url))
            .header("X-Request-ID", request_id.to_string())
            .json(&RedeemConnectionRequest { external_tenant_id: school_id, connection_code: code, external_actor_id: actor })
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time connection request failed: {e}")))?
            .error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected connection request: {e}")))?
            .json::<TapTimeRemoteConnection>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time connection response: {e}")))
    }

    async fn delete_remote_connection(&self, school_id: Uuid) -> Result<(), AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4();
        let token = self.connection_token(school_id).await?;
        self.http.delete(format!("{}/integrations/v1/tenants/{}/connection", base_url, school_id))
            .bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time disconnect request failed: {e}")))?
            .error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected disconnect request: {e}")))?;
        Ok(())
    }

    async fn upsert_remote_employee(&self, employee: &EmployeeSyncData, actor: Uuid) -> Result<TapTimeRemoteEmployee, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4();
        let token = self.connection_token(employee.school_id).await?;
        let response = self.http.put(format!("{}/integrations/v1/tenants/{}/employees/{}", base_url, employee.school_id, employee.id))
            .bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .json(&EmployeeUpsertRequest { first_name: &employee.first_name, last_name: &employee.last_name, phone_number: employee.phone.as_deref(), email: Some(&employee.email), is_active: employee.is_active, is_admin: employee.is_admin, external_actor_id: actor })
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time employee sync request failed: {e}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::CONFLICT {
                return Err(AppError::Conflict(format!("Tap-Time employee already exists; review an exact phone or email match before syncing. {body}")));
            }
            return Err(AppError::ExternalService(format!("Tap-Time rejected employee sync ({status}): {body}")));
        }
        response.json::<TapTimeRemoteEmployee>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time employee sync response: {e}")))
    }

    async fn update_remote_pin(&self, school_id: Uuid, employee_id: Uuid, pin: &str, actor: Uuid) -> Result<(), AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4();
        let token = self.connection_token(school_id).await?;
        self.http.post(format!("{}/integrations/v1/tenants/{}/employees/{}/pin", base_url, school_id, employee_id))
            .bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .json(&PinUpdateRequest { pin, external_actor_id: actor })
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time PIN update request failed: {e}")))?
            .error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected PIN update: {e}")))?;
        Ok(())
    }

    async fn remote_pin(&self, school_id: Uuid, external_id: Uuid) -> Result<TapTimePinResponse, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4();
        let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/employees/{}/pin", base_url, school_id, external_id))
            .bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time PIN read request failed: {e}")))?
            .error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected PIN read: {e}")))?
            .json::<TapTimePinResponse>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time PIN response: {e}")))
    }

    async fn remote_pins(&self, school_id: Uuid) -> Result<TapTimeRemotePinsResponse, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/pins", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time PIN list request failed: {e}")))?
            .error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected PIN list: {e}")))?
            .json::<TapTimeRemotePinsResponse>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time PIN list response: {e}")))
    }

    async fn remote_employee_candidates(&self, school_id: Uuid) -> Result<Vec<TapTimeEmployeeCandidate>, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/employees", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time employee list request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected employee list: {e}")))?
            .json::<Vec<TapTimeEmployeeCandidate>>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time employee list response: {e}")))
    }

    async fn reconcile_remote_employee(&self, school_id: Uuid, employee_id: Uuid, tap_employee_id: Uuid, actor: Uuid) -> Result<(), AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.post(format!("{}/integrations/v1/tenants/{}/employees/reconcile", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .json(&ReconcileEmployeeRequest { external_employee_id: employee_id, internal_employee_id: tap_employee_id, external_actor_id: actor })
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time reconcile request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected reconcile request: {e}")))?;
        Ok(())
    }

    async fn remote_reports(&self, school_id: Uuid, report_date: Option<chrono::NaiveDate>, start_date: Option<chrono::NaiveDate>, end_date: Option<chrono::NaiveDate>, employee_id: Option<Uuid>, pending_checkout: Option<bool>) -> Result<Vec<TapTimeReport>, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        let mut query = Vec::new();
        if let Some(value) = report_date { query.push(("report_date", value.to_string())); }
        if let Some(value) = start_date { query.push(("start_date", value.to_string())); }
        if let Some(value) = end_date { query.push(("end_date", value.to_string())); }
        if let Some(value) = employee_id { query.push(("employee_external_id", value.to_string())); }
        if let Some(value) = pending_checkout { query.push(("pending_checkout", value.to_string())); }
        self.http.get(format!("{}/integrations/v1/tenants/{}/reports/daily", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string()).query(&query)
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report list request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report list: {e}")))?
            .json::<Vec<TapTimeReport>>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time report list response: {e}")))
    }

    async fn remote_report_people(&self, school_id: Uuid) -> Result<Vec<TapTimeReportPerson>, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/report-people", base_url, school_id)).bearer_auth(token).header("X-Request-ID", Uuid::new_v4().to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report people request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report people request: {e}")))?
            .json::<Vec<TapTimeReportPerson>>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time report people response: {e}")))
    }

    async fn remote_create_report(&self, school_id: Uuid, request: &CreateTimeReportRequest, actor: Uuid) -> Result<TapTimeReport, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let token = self.connection_token(school_id).await?;
        self.http.post(format!("{}/integrations/v1/tenants/{}/reports", base_url, school_id)).bearer_auth(token).header("X-Request-ID", Uuid::new_v4().to_string())
            .json(&RemoteReportCreate { external_employee_id: request.external_employee_id, check_in_time: request.check_in_time, check_out_time: request.check_out_time, reason: request.reason.trim(), external_actor_id: actor })
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report create request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report create: {e}")))?
            .json::<TapTimeReport>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time report create response: {e}")))
    }

    async fn summary_request(&self, school_id: Uuid, path: &str, query: Vec<(&str, String)>) -> Result<reqwest::Response, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/reports/{}", base_url, school_id, path)).bearer_auth(token).header("X-Request-ID", request_id.to_string()).query(&query)
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report summary request failed: {e}")))?
            .error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report summary: {e}")))
    }

    async fn remote_summary(&self, school_id: Uuid, path: &str, report_date: Option<chrono::NaiveDate>) -> Result<TapTimeReportOverview, AppError> {
        let mut query = Vec::new(); if let Some(value) = report_date { query.push(("report_date", value.to_string())); }
        self.summary_request(school_id, path, query).await?.json::<TapTimeReportOverview>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time overview response: {e}")))
    }

    async fn remote_two_day_summary(&self, school_id: Uuid, report_date: Option<chrono::NaiveDate>) -> Result<TapTimeTwoDayReport, AppError> {
        let mut query = Vec::new(); if let Some(value) = report_date { query.push(("report_date", value.to_string())); }
        self.summary_request(school_id, "two-day", query).await?.json::<TapTimeTwoDayReport>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time two-day response: {e}")))
    }

    async fn remote_salary_summary(&self, school_id: Uuid, anchor_date: Option<chrono::NaiveDate>) -> Result<Vec<TapTimeSalaryPeriod>, AppError> {
        let mut query = Vec::new(); if let Some(value) = anchor_date { query.push(("anchor_date", value.to_string())); }
        self.summary_request(school_id, "salary", query).await?.json::<Vec<TapTimeSalaryPeriod>>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time salary response: {e}")))
    }

    async fn remote_consolidated_report(&self, school_id: Uuid, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> Result<Vec<TapTimeConsolidatedReport>, AppError> {
        self.summary_request(school_id, "consolidated", vec![("start_date", start_date.to_string()), ("end_date", end_date.to_string())]).await?
            .json::<Vec<TapTimeConsolidatedReport>>().await
            .map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time consolidated report response: {e}")))
    }

    async fn remote_day_trends(&self, school_id: Uuid, start_date: Option<chrono::NaiveDate>, end_date: Option<chrono::NaiveDate>) -> Result<Vec<TapTimeDayTrend>, AppError> {
        let mut query = Vec::new(); if let Some(value) = start_date { query.push(("start_date", value.to_string())); } if let Some(value) = end_date { query.push(("end_date", value.to_string())); }
        self.summary_request(school_id, "day-trends", query).await?.json::<Vec<TapTimeDayTrend>>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time day-trends response: {e}")))
    }

    async fn remote_report_settings(&self, school_id: Uuid) -> Result<Vec<TapTimeReportSetting>, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/report-settings", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report-settings request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report-settings request: {e}")))?
            .json::<Vec<TapTimeReportSetting>>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time report-settings response: {e}")))
    }

    async fn remote_create_report_setting(&self, school_id: Uuid, request: &UpsertTimeReportSettingRequest, actor: Uuid) -> Result<TapTimeReportSetting, AppError> {
        self.remote_upsert_report_setting(reqwest::Method::POST, school_id, None, request, actor).await
    }

    async fn remote_update_report_setting(&self, school_id: Uuid, setting_id: Uuid, request: &UpsertTimeReportSettingRequest, actor: Uuid) -> Result<TapTimeReportSetting, AppError> {
        self.remote_upsert_report_setting(reqwest::Method::PATCH, school_id, Some(setting_id), request, actor).await
    }

    async fn remote_upsert_report_setting(&self, method: reqwest::Method, school_id: Uuid, setting_id: Option<Uuid>, request: &UpsertTimeReportSettingRequest, actor: Uuid) -> Result<TapTimeReportSetting, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        let url = match setting_id { Some(id) => format!("{}/integrations/v1/tenants/{}/report-settings/{}", base_url, school_id, id), None => format!("{}/integrations/v1/tenants/{}/report-settings", base_url, school_id) };
        self.http.request(method, url).bearer_auth(token).header("X-Request-ID", request_id.to_string()).json(&RemoteReportSettingUpsert {
            reporter_email: request.reporter_email.trim(), is_daily_report_active: request.is_daily_report_active, is_weekly_report_active: request.is_weekly_report_active,
            is_bi_weekly_report_active: request.is_bi_weekly_report_active, is_monthly_report_active: request.is_monthly_report_active,
            is_bi_monthly_report_active: request.is_bi_monthly_report_active, external_actor_id: actor,
        }).send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report-settings write failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report-settings write: {e}")))?
            .json::<TapTimeReportSetting>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time report-settings response: {e}")))
    }

    async fn remote_delete_report_setting(&self, school_id: Uuid, setting_id: Uuid) -> Result<(), AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.delete(format!("{}/integrations/v1/tenants/{}/report-settings/{}", base_url, school_id, setting_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time report-settings delete failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected report-settings delete: {e}")))?;
        Ok(())
    }

    async fn remote_consolidated_report_setting(&self, school_id: Uuid) -> Result<TapTimeConsolidatedReportSetting, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.get(format!("{}/integrations/v1/tenants/{}/report-settings/consolidated", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time consolidated report-setting request failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected consolidated report-setting request: {e}")))?
            .json::<TapTimeConsolidatedReportSetting>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time consolidated report-setting response: {e}")))
    }

    async fn remote_update_consolidated_report_setting(&self, school_id: Uuid, report_type: &str, actor: Uuid) -> Result<TapTimeConsolidatedReportSetting, AppError> {
        let base_url = self.config.as_ref().ok_or_else(|| AppError::Validation("Tap-Time integration is not configured".to_string()))?.base_url.clone();
        let request_id = Uuid::new_v4(); let token = self.connection_token(school_id).await?;
        self.http.put(format!("{}/integrations/v1/tenants/{}/report-settings/consolidated", base_url, school_id)).bearer_auth(token).header("X-Request-ID", request_id.to_string())
            .json(&serde_json::json!({ "report_type": report_type.trim(), "external_actor_id": actor }))
            .send().await.map_err(|e| AppError::ExternalService(format!("Tap-Time consolidated report-setting update failed: {e}")))?.error_for_status().map_err(|e| AppError::ExternalService(format!("Tap-Time rejected consolidated report-setting update: {e}")))?
            .json::<TapTimeConsolidatedReportSetting>().await.map_err(|e| AppError::ExternalService(format!("Invalid Tap-Time consolidated report-setting response: {e}")))
    }
}

fn normalize_phone(value: &str) -> Option<String> {
    let digits: String = value.chars().filter(|character| character.is_ascii_digit()).collect();
    if digits.is_empty() { None } else { Some(digits) }
}

fn normalize_email(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() { None } else { Some(normalized) }
}

fn reconciliation_proposal(employee: &EmployeeSyncData, candidate: &TapTimeEmployeeCandidate, match_type: &str, match_value: &str) -> ReconciliationProposal {
    ReconciliationProposal {
        employee_id: employee.id,
        employee_name: format!("{} {}", employee.first_name, employee.last_name),
        entity_type: if employee.is_admin == 0 { "employee" } else { "user" }.to_string(),
        role: if employee.is_admin == 2 { "Super Admin" } else if employee.is_admin == 1 { "Admin" } else { "Employee" }.to_string(),
        match_type: match_type.to_string(),
        match_value: match_value.to_string(),
        tap_employee_id: candidate.internal_employee_id,
        tap_employee_name: format!("{} {}", candidate.first_name, candidate.last_name),
        normalized_phone: candidate.phone_number.as_deref().and_then(normalize_phone).unwrap_or_default(),
    }
}
