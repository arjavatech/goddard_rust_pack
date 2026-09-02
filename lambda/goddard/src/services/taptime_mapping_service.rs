use crate::{
    dao::{SchoolDao, TapTimeDatabaseDiagnostics, TapTimeMappingDao},
    error::{ApiResult, AppError},
    middleware::auth::{check_permission_admin_or_superadmin, check_permission_superadmin_only, AuthContext},
    services::{SupabaseClient, TapTimeService},
};
use crate::models::school::SchoolFeature;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct MappingUser {
    pub user_id: String,
    pub employee_id: Option<String>,
    pub school_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub mapping_status: String,
    pub taptime_employee: Option<serde_json::Value>,
    pub last_push_at: Option<String>,
    pub last_push_error: Option<String>,
}
#[derive(Serialize)]
pub struct AttendanceUser {
    pub external_employee_id: String,
    pub taptime_employee_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
}
#[derive(Deserialize)]
pub struct CreateMappingRequest {
    pub school_id: Uuid,
    pub goddard_user_id: Uuid,
    pub pin: String,
}
#[derive(Deserialize)]
pub struct RedeemPairingCodeRequest {
    pub school_id: Uuid,
    pub code: String,
}
#[derive(Serialize)]
pub struct TapTimeSetupStatus {
    pub school_id: String,
    pub configured: bool,
    pub message: String,
}
#[derive(Serialize)]
pub struct TapTimeIntegrationStatus {
    pub school_id: String,
    pub configured: bool,
    pub message: String,
    pub active_staff_count: usize,
    pub connected_user_count: usize,
    pub unresolved_user_count: usize,
}
#[derive(Serialize)]
pub struct TapTimeReconciliationResult {
    pub linked_count: usize,
    pub already_connected_count: usize,
    pub unresolved_count: usize,
    pub failed_user_ids: Vec<String>,
}
#[derive(Serialize)]
pub struct TapTimeAccessSyncResult {
    pub updated_users: usize,
    pub failed_users: Vec<String>,
}
#[derive(Deserialize)]
pub struct UpdateTapTimeSettingsRequest {
    pub default_report_type: String,
}
#[derive(Serialize)]
pub struct TapTimeSettingsResponse {
    pub default_report_type: Option<String>,
    pub employment_types: Vec<String>,
}

#[derive(Clone)]
pub struct TapTimeMappingService {
    mappings: TapTimeMappingDao,
    taptime: TapTimeService,
    supabase: SupabaseClient,
    schools: SchoolDao,
}
impl TapTimeMappingService {
    async fn ensure_enabled(&self, school_id: Uuid) -> ApiResult<()> {
        self.schools.ensure_feature_enabled(school_id, SchoolFeature::TapTime).await
    }
    pub fn new(
        mappings: TapTimeMappingDao,
        taptime: TapTimeService,
        supabase: SupabaseClient,
        schools: SchoolDao,
    ) -> Self {
        Self {
            mappings,
            taptime,
            supabase,
            schools,
        }
    }
    pub async fn users(&self, school_id: Uuid) -> ApiResult<Vec<MappingUser>> {
        self.ensure_enabled(school_id).await?;
        Ok(self.mappings.eligible_users(school_id).await?.into_iter().map(|user| {
            let mapped = user.taptime_employee_id.is_some();
            MappingUser {
                user_id: user.user_id.to_string(),
                employee_id: user.employee_id.map(|value| value.to_string()),
                school_id: user.school_id.to_string(),
                first_name: user.first_name,
                last_name: user.last_name,
                email: user.email,
                phone: user.phone,
                role: user.role,
                is_active: user.is_active,
                mapping_status: if mapped { "connected".into() } else { "not_connected".into() },
                taptime_employee: user.taptime_employee_id.map(|emp_id| serde_json::json!({"emp_id": emp_id})),
                last_push_at: None,
                last_push_error: None,
            }
        }).collect())
    }

    /// Fast, local roster used by the attendance form. The external Goddard
    /// user ID remains the only identifier sent by the browser to TapTime.
    pub async fn attendance_users(&self, school_id: Uuid) -> ApiResult<Vec<AttendanceUser>> {
        self.ensure_enabled(school_id).await?;
        Ok(self.mappings.eligible_users(school_id).await?.into_iter().filter_map(|user| {
            user.taptime_employee_id.map(|taptime_employee_id| AttendanceUser {
                external_employee_id: user.user_id.to_string(),
                taptime_employee_id: taptime_employee_id.to_string(),
                first_name: user.first_name,
                last_name: user.last_name,
                email: user.email,
                role: user.role,
            })
        }).collect())
    }

    /// Updates TapTime directly, then mirrors the confirmed value locally.
    pub async fn update_employee_pin(
        &self,
        actor: &AuthContext,
        school_id: Uuid,
        goddard_user_id: Uuid,
        pin: &str,
    ) -> ApiResult<()> {
        self.ensure_enabled(school_id).await?;
        check_permission_admin_or_superadmin(actor, &school_id)?;
        if actor.user_id != Uuid::nil() && actor.school_id != school_id {
            return Err(AppError::Authorization("Access denied to school".into()));
        }
        if pin.len() != 4 || !pin.chars().all(|value| value.is_ascii_digit()) {
            return Err(AppError::Validation("PIN must contain exactly 4 digits".into()));
        }

        let user = self.mappings.eligible_users(school_id).await?.into_iter()
            .find(|item| item.user_id == goddard_user_id)
            .ok_or_else(|| AppError::NotFound("Goddard user".into()))?;
        if matches!(user.role.as_str(), "Admin" | "SuperAdmin") {
            check_permission_superadmin_only(actor)?;
        }
        if user.taptime_employee_id.is_none() {
            return Err(AppError::Conflict("This user is not connected to TapTime".into()));
        }

        self.mappings.ensure_pin_available(school_id, goddard_user_id, pin).await?;
        self.taptime.deliver(
            school_id,
            goddard_user_id,
            "pin",
            &serde_json::json!({ "pin": pin }),
            Uuid::new_v4(),
        ).await?;
        self.mappings.save_pin(goddard_user_id, pin).await
    }

    /// Mirrors a PIN that the authenticated browser has already changed using
    /// TapTime's self-only integration endpoint. This performs no external
    /// call: TapTime remains the authority and this database stores only the
    /// value needed to render the user's own profile promptly.
    pub async fn mirror_own_pin(&self, actor: &AuthContext, pin: &str) -> ApiResult<()> {
        if actor.user_id == Uuid::nil() {
            return Err(AppError::Authorization("A signed-in user is required".into()));
        }
        self.ensure_enabled(actor.school_id).await?;
        if pin.len() != 4 || !pin.chars().all(|value| value.is_ascii_digit()) {
            return Err(AppError::Validation("PIN must contain exactly 4 digits".into()));
        }
        let user = self.mappings.eligible_users(actor.school_id).await?.into_iter()
            .find(|item| item.user_id == actor.user_id)
            .ok_or_else(|| AppError::NotFound("Goddard user".into()))?;
        if user.taptime_employee_id.is_none() || user.taptime_pin.is_none() {
            return Err(AppError::Conflict("This user is not connected to TapTime".into()));
        }
        self.mappings.save_pin(actor.user_id, pin).await
    }
    pub async fn available_taptime_users(
        &self,
        school_id: Uuid,
    ) -> ApiResult<Vec<serde_json::Value>> {
        self.ensure_enabled(school_id).await?;
        let mut users = self.taptime.available_employees(school_id).await?;
        for user in &mut users {
            let emp_id = user
                .get("emp_id")
                .and_then(serde_json::Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok());
            let role_label = match user
                .get("is_admin")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0)
            {
                2 => "Super Admin",
                1 => "Admin",
                _ => "Employee",
            };
            if let Some(object) = user.as_object_mut() {
                object.insert("role_label".into(), serde_json::Value::String(role_label.into()));
                object.insert(
                    "mapping_status".into(),
                    serde_json::Value::String(
                        if emp_id.is_some() { "available" } else { "unknown" }
                        .into(),
                    ),
                );
            }
        }

        Ok(users)
    }

    pub async fn database_diagnostics(&self) -> ApiResult<TapTimeDatabaseDiagnostics> {
        self.mappings.database_diagnostics().await
    }

    pub async fn setup_status(&self, school_id: Uuid) -> ApiResult<TapTimeSetupStatus> {
        self.ensure_enabled(school_id).await?;
        // A provisioning-only lookup is also a safe connection check: it can
        // return an empty employee list while still proving the tenant link.
        match self.taptime.available_employees(school_id).await {
            Ok(_) => Ok(TapTimeSetupStatus {
                school_id: school_id.to_string(), configured: true,
                message: "TapTime is connected for this school".into(),
            }),
            Err(_) => Ok(TapTimeSetupStatus {
                school_id: school_id.to_string(), configured: false,
                message: "TapTime has not been connected for this school".into(),
            }),
        }
    }

    /// A safe, customer-facing verification summary. Counts are calculated
    /// from Goddard's active staff and persisted TapTime identities; no
    /// TapTime credentials, company IDs, or employee records are exposed.
    pub async fn integration_status(&self, school_id: Uuid) -> ApiResult<TapTimeIntegrationStatus> {
        let setup = self.setup_status(school_id).await?;
        let users = self.users(school_id).await?;
        let active_users: Vec<_> = users.into_iter().filter(|user| user.is_active).collect();
        let connected_user_count = active_users
            .iter()
            .filter(|user| user.mapping_status == "connected")
            .count();
        let active_staff_count = active_users.len();
        Ok(TapTimeIntegrationStatus {
            school_id: setup.school_id,
            configured: setup.configured,
            message: setup.message,
            active_staff_count,
            connected_user_count,
            unresolved_user_count: active_staff_count.saturating_sub(connected_user_count),
        })
    }

    pub async fn redeem_pairing_code(&self, request: RedeemPairingCodeRequest) -> ApiResult<TapTimeSetupStatus> {
        self.ensure_enabled(request.school_id).await?;
        if request.code.trim().is_empty() {
            return Err(AppError::Validation("A TapTime linking code is required".into()));
        }
        self.taptime.redeem_tenant_pairing_code(request.school_id, request.code.trim()).await?;
        let linked = self.reconcile_email_matches(request.school_id).await?;
        // Pairing is the single self-service activation step.  Refresh the
        // server-owned JWT claims for existing school staff here, rather than
        // requiring an administrator to run a separate access-sync operation.
        let access_sync = self.sync_access(request.school_id).await?;
        Ok(TapTimeSetupStatus {
            school_id: request.school_id.to_string(), configured: true,
            message: format!(
                "TapTime is connected. {linked} existing email matches were connected and access claims were refreshed for {} users. Users must refresh their session or sign in again before using TapTime.",
                access_sync.updated_users,
            ),
        })
    }

    /// Resolve only exact, company-scoped email matches.  This operation never
    /// creates TapTime records and never guesses when TapTime contains duplicates.
    pub async fn reconcile_email_matches(&self, school_id: Uuid) -> ApiResult<usize> {
        self.ensure_enabled(school_id).await?;
        let taptime_users = self.taptime.available_employees(school_id).await?;
        let mut linked = 0;
        for user in self.mappings.eligible_users(school_id).await? {
            if user.taptime_employee_id.is_some() { continue; }
            let matches: Vec<&serde_json::Value> = taptime_users.iter().filter(|candidate| {
                candidate.get("email").and_then(serde_json::Value::as_str)
                    .is_some_and(|email| email.eq_ignore_ascii_case(user.email.trim()))
            }).collect();
            if matches.len() != 1 { continue; }
            let candidate = matches[0];
            let emp_id = candidate.get("emp_id").and_then(serde_json::Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok());
            let pin = candidate.get("pin").and_then(serde_json::Value::as_str);
            if let (Some(emp_id), Some(pin)) = (emp_id, pin) {
                self.taptime.link_existing_employee(school_id, user.user_id, emp_id).await?;
                self.mappings.save_identity(user.user_id, emp_id, pin).await?;
                self.supabase.set_taptime_access_claims(user.user_id, school_id, &user.role).await?;
                linked += 1;
            }
        }
        Ok(linked)
    }

    /// Reconcile one Goddard user after a staff record is created or updated.
    /// This is deliberately best-effort at its call sites: Goddard staff
    /// operations must not fail just because TapTime is temporarily unavailable.
    /// It only links one exact email match and never creates a TapTime user.
    pub async fn reconcile_user_email(&self, school_id: Uuid, user_id: Uuid) -> ApiResult<bool> {
        self.ensure_enabled(school_id).await?;
        if !self.setup_status(school_id).await?.configured {
            return Ok(false);
        }
        let user = self.mappings.eligible_users(school_id).await?
            .into_iter()
            .find(|candidate| candidate.user_id == user_id);
        let Some(user) = user else { return Ok(false); };
        if user.taptime_employee_id.is_some() { return Ok(true); }

        let matches: Vec<serde_json::Value> = self.taptime.available_employees(school_id).await?
            .into_iter()
            .filter(|candidate| candidate.get("email").and_then(serde_json::Value::as_str)
                .is_some_and(|email| email.eq_ignore_ascii_case(user.email.trim())))
            .collect();
        if matches.len() != 1 { return Ok(false); }
        let candidate = &matches[0];
        let emp_id = candidate.get("emp_id").and_then(serde_json::Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        let pin = candidate.get("pin").and_then(serde_json::Value::as_str);
        if let (Some(emp_id), Some(pin)) = (emp_id, pin) {
            self.taptime.link_existing_employee(school_id, user.user_id, emp_id).await?;
            self.mappings.save_identity(user.user_id, emp_id, pin).await?;
            self.supabase.set_taptime_access_claims(user.user_id, school_id, &user.role).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn reconcile_with_summary(&self, school_id: Uuid) -> ApiResult<TapTimeReconciliationResult> {
        if !self.setup_status(school_id).await?.configured {
            return Err(AppError::Conflict("Connect this school to TapTime before syncing email matches".into()));
        }
        let before = self.users(school_id).await?;
        let already_connected_count = before.iter()
            .filter(|user| user.is_active && user.mapping_status == "connected")
            .count();
        let linked_count = self.reconcile_email_matches(school_id).await?;
        let after = self.users(school_id).await?;
        let unresolved_count = after.iter()
            .filter(|user| user.is_active && user.mapping_status != "connected")
            .count();
        Ok(TapTimeReconciliationResult {
            linked_count,
            already_connected_count,
            unresolved_count,
            failed_user_ids: Vec::new(),
        })
    }

    pub async fn sync_access(&self, school_id: Uuid) -> ApiResult<TapTimeAccessSyncResult> {
        // Do not issue TapTime access merely because an account exists.  The
        // school must first be linked, and only active school staff are synced.
        if !self.setup_status(school_id).await?.configured {
            return Err(AppError::Conflict("Connect this school to TapTime before syncing user access".into()));
        }
        // The TapTime roster intentionally includes Employees, Admins, and
        // Super Admins. Keep this independent from /users/admin, whose UI is
        // correctly limited to Admin records only.
        let users: Vec<(Uuid, String)> = self.mappings.eligible_users(school_id).await?
            .into_iter().map(|user| (user.user_id, user.role)).collect();
        let mut updated_users = 0;
        let mut failed_users = Vec::new();
        for (user_id, role) in users {
            match self.supabase.set_taptime_access_claims(user_id, school_id, &role).await {
                Ok(()) => updated_users += 1,
                Err(error) => {
                    tracing::warn!(%user_id, %role, error = %error, "Failed to sync TapTime user claims");
                    failed_users.push(user_id.to_string());
                }
            }
        }
        Ok(TapTimeAccessSyncResult { updated_users, failed_users })
    }

    pub async fn settings(&self, school_id: Uuid) -> ApiResult<TapTimeSettingsResponse> {
        self.ensure_enabled(school_id).await?;
        let default_report_type = self.schools.get_taptime_default_report_type(school_id).await?;
        let employment_types = self.taptime.employment_types(school_id).await?;
        Ok(TapTimeSettingsResponse { default_report_type, employment_types })
    }

    pub async fn update_settings(&self, school_id: Uuid, request: UpdateTapTimeSettingsRequest) -> ApiResult<TapTimeSettingsResponse> {
        self.ensure_enabled(school_id).await?;
        let selected = request.default_report_type.trim();
        if selected.is_empty() { return Err(AppError::Validation("A default report type is required".into())); }
        let employment_types = self.taptime.employment_types(school_id).await?;
        if !employment_types.iter().any(|item| item == selected) {
            return Err(AppError::Validation("Default report type must be a configured TapTime employment type".into()));
        }
        self.schools.set_taptime_default_report_type(school_id, selected).await?;
        Ok(TapTimeSettingsResponse { default_report_type: Some(selected.to_string()), employment_types })
    }
    pub async fn create_mapping(
        &self,
        request: CreateMappingRequest,
        mapped_by: Uuid,
    ) -> ApiResult<()> {
        let user = self
            .users(request.school_id)
            .await?
            .into_iter()
            .find(|item| item.user_id == request.goddard_user_id.to_string())
            .ok_or_else(|| AppError::NotFound("Eligible Goddard user".into()))?;
        if user.mapping_status == "connected" {
            return Err(AppError::Conflict(
                "This Goddard user is already mapped".into(),
            ));
        }
        let phone = user.phone.as_deref().map(str::trim).filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Validation("A phone number is required before creating a TapTime user".into()))?;
        if request.pin.len() != 4 || !request.pin.chars().all(|value| value.is_ascii_digit()) {
            return Err(AppError::Validation("PIN must contain exactly 4 digits".into()));
        }
        // A phone-derived PIN is just the initial suggestion.  If it is already
        // used by this TapTime company, choose a free four-digit alternative.
        let existing_pins: std::collections::HashSet<String> = self.taptime.available_employees(request.school_id).await?
            .iter().filter_map(|item| item.get("pin").and_then(serde_json::Value::as_str).map(str::to_string)).collect();
        let mut pin = request.pin.clone();
        if existing_pins.contains(&pin) {
            for _ in 0..10_000 {
                let candidate = format!("{:04}", Uuid::new_v4().as_u128() % 10_000);
                if !existing_pins.contains(&candidate) { pin = candidate; break; }
            }
            if existing_pins.contains(&pin) { return Err(AppError::Conflict("No free four-digit TapTime PIN is available".into())); }
        }
        let role = match user.role.as_str() { "Employee" => "employee", "Admin" => "admin", "SuperAdmin" => "super_admin", _ => return Err(AppError::Validation("Unsupported Goddard role".into())) };
        let response = self.taptime.deliver(request.school_id, request.goddard_user_id, "upsert", &serde_json::json!({
            "first_name": user.first_name, "last_name": user.last_name, "email": user.email,
            "phone_number": phone, "pin": pin, "role": role,
            "is_active": true, "external_auth_subject": request.goddard_user_id.to_string(),
        }), Uuid::new_v4()).await?;
        let emp_id = response.get("data").and_then(|value| value.get("emp_id")).and_then(serde_json::Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| AppError::ExternalService("TapTime did not return an employee ID".into()))?;
        self.mappings.save_identity(request.goddard_user_id, emp_id, &pin).await?;
        self.supabase.set_taptime_access_claims(request.goddard_user_id, request.school_id, &user.role).await?;
        Ok(())
    }
}
