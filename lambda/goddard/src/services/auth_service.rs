use crate::{
    dao::AuthDao,
    error::{AppError, ApiResult},
    utils::ValidationUtils,
    services::{SupabaseClient, supabase_client::UserMetadata},
    models::school::SchoolResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid;

#[derive(Debug, Serialize)]
pub struct AuthVerificationResponse {
    pub total_users: i64,
    pub confirmed_users: i64,
    pub invited_not_confirmed: i64,
    pub confirmation_sent_not_confirmed: i64,
    pub users_who_signed_in: i64,
    pub verification_rate: f64,
    pub timestamp: DateTime<Utc>,
    pub details: Vec<UserAuthStatus>,
}

#[derive(Debug, Serialize)]
pub struct UserAuthStatus {
    pub email: String,
    pub status: String,
    pub invited_at: Option<DateTime<Utc>>,
    pub confirmation_sent_at: Option<DateTime<Utc>>,
    pub email_confirmed_at: Option<DateTime<Utc>>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct InvitationSummary {
    pub total_invitations_sent: i64,
    pub pending_confirmations: i64,
    pub completed_signups: i64,
    pub expired_invitations: i64,
    pub by_role: RoleBreakdown,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RoleBreakdown {
    pub super_admin: i64,
    pub admin: i64,
    pub teacher: i64,
    pub parent: i64,
}

#[derive(Debug, Deserialize)]
pub struct ResendInvitationRequest {
    pub email: String,
    pub school_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResendInvitationResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
    pub email_status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub school_id: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,  // Only phone_number is optional
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequestEnhanced {
    pub email: String,
    pub school_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSuperAdminRequest {
    pub email: String,
    pub school_id: String,           // UUID format
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateInvitationResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
    pub user_id: String,
    pub email_status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminUserResponse {
    pub id: String,
    pub school_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
    pub is_verified: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAdminRequest {
    pub user_id: Option<String>,  // Optional - SuperAdmin can pass to update other admins
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteAdminRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResendAdminInviteRequest {
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Serialize)]
pub struct ResendAdminInviteResponse {
    pub user_id: String,
    pub email: String,
    pub email_sent: bool,
    pub message: String,
    pub email_status: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct ForgotPasswordResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct FilteredUserResponse {
    pub school_id: String,
    pub role: String,
    pub user_id: String,
    pub email: String,
    pub parent_id: Option<String>,
    pub last_name: Option<String>,
    pub first_name: Option<String>,
    pub school_data: Option<SchoolResponse>,
}

fn email_status_message(status: &str, default_ok: &str) -> String {
    match status {
        "suppressed" => "Email was suppressed by the mail provider. The address may have previously bounced — please ask the recipient to check with their IT or try a different address.".to_string(),
        "bounced"    => "Email bounced. Please verify the email address is correct and able to receive mail.".to_string(),
        "delivered"  => format!("{} Email delivered successfully.", default_ok),
        _            => default_ok.to_string(),
    }
}

pub struct AuthService {
    dao: AuthDao,
    school_dao: crate::dao::school_dao::SchoolDao,
    supabase_client: SupabaseClient,
    notification_service: std::sync::Arc<crate::services::NotificationService>,
}

impl AuthService {
    pub fn new(
        dao: AuthDao,
        school_dao: crate::dao::school_dao::SchoolDao,
        supabase_client: SupabaseClient,
        notification_service: std::sync::Arc<crate::services::NotificationService>,
    ) -> Self {
        Self {
            dao,
            school_dao,
            supabase_client,
            notification_service,
        }
    }

    pub async fn get_auth_verification_status(
        &self,
        school_id: Option<String>,
        include_details: Option<bool>,
    ) -> ApiResult<AuthVerificationResponse> {
        // Validate school_id if provided
        if let Some(ref school_id_str) = school_id {
            ValidationUtils::validate_uuid(school_id_str)?;
        }

        // Get statistics
        let stats = self.dao.get_auth_verification_stats().await?;

        // Calculate verification rate
        let verification_rate = if stats.total_users > 0 {
            (stats.confirmed_users as f64 / stats.total_users as f64) * 100.0
        } else {
            0.0
        };

        // Get detailed user information if requested
        let details = if include_details.unwrap_or(true) {
            let user_details = self.dao.get_user_details(school_id).await?;
            user_details
                .into_iter()
                .map(|user| UserAuthStatus {
                    email: user.email.unwrap_or_default(),
                    status: user.status.unwrap_or_default(),
                    invited_at: user.invited_at,
                    confirmation_sent_at: user.confirmation_sent_at,
                    email_confirmed_at: user.email_confirmed_at,
                    last_sign_in_at: user.last_sign_in_at,
                    created_at: user.created_at.unwrap_or_else(|| Utc::now()),
                })
                .collect()
        } else {
            vec![]
        };

        Ok(AuthVerificationResponse {
            total_users: stats.total_users,
            confirmed_users: stats.confirmed_users,
            invited_not_confirmed: stats.invited_not_confirmed,
            confirmation_sent_not_confirmed: stats.confirmation_sent_not_confirmed,
            users_who_signed_in: stats.users_who_signed_in,
            verification_rate,
            timestamp: Utc::now(),
            details,
        })
    }

    pub async fn get_invitation_summary(&self) -> ApiResult<InvitationSummary> {
        let stats = self.dao.get_auth_verification_stats().await?;
        let (super_admin, admin, teacher, parent) = self.dao.get_invitation_summary_by_role().await?;

        Ok(InvitationSummary {
            total_invitations_sent: stats.total_users,
            pending_confirmations: stats.invited_not_confirmed,
            completed_signups: stats.confirmed_users,
            expired_invitations: 0, // TODO: Add logic for expired invitations
            by_role: RoleBreakdown {
                super_admin,
                admin,
                teacher,
                parent,
            },
            timestamp: Utc::now(),
        })
    }

    pub async fn resend_invitation(&self, request: ResendInvitationRequest) -> ApiResult<ResendInvitationResponse> {
        // Validate email format
        ValidationUtils::validate_email(&request.email)?;

        // Validate school_id if provided
        if let Some(ref school_id) = request.school_id {
            ValidationUtils::validate_uuid(school_id)?;
        }

        // Check if user exists
        if !self.dao.user_exists_by_email(&request.email).await? {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        // Check if user needs confirmation
        if !self.dao.user_needs_confirmation(&request.email).await? {
            return Err(AppError::Conflict("User already confirmed".to_string()));
        }

        // Try to send email via Supabase, but handle rate limiting gracefully
        match self.supabase_client.resend_invitation(&request.email).await {
            Ok(_) => {
                self.dao.update_confirmation_sent_at(&request.email).await?;
            }
            Err(AppError::ExternalService(msg)) if msg.contains("rate limit") => {
                self.dao.update_confirmation_sent_at(&request.email).await?;
            }
            Err(e) => return Err(e),
        }

        let email_status = self.supabase_client.get_recent_email_status(&request.email).await;

        if matches!(email_status.as_str(), "suppressed" | "bounced") {
            return Err(AppError::ExternalService(email_status_message(&email_status, "")));
        }

        let message = email_status_message(&email_status, "Invitation processed successfully. If no email was received, please check spam folder.");

        Ok(ResendInvitationResponse {
            success: true,
            message,
            email: request.email,
            email_status,
            timestamp: Utc::now(),
        })
    }

    /// Fire the "New Admin Added" in-app notification to all other active admins
    /// of the school. Called from both `create_invitation` and `create_invitation_enhanced`
    /// so the wiring can't drift between the two routes.
    async fn fire_admin_added_notification(
        &self,
        school_id: uuid::Uuid,
        new_admin_user_id: &str,
        first_name: &str,
        last_name: &str,
        role: &str,
        school_name: &str,
    ) {
        let parsed_id = uuid::Uuid::parse_str(new_admin_user_id).ok();
        self.notification_service.notify_school_admins(
            crate::models::notification::CreateNotification {
                school_id,
                notification_type: crate::models::notification::notification_type::ADMIN_ADDED.to_string(),
                title: "New Admin Added".to_string(),
                body: format!(
                    "{} {} has been added as {} for {}.",
                    first_name.trim(),
                    last_name.trim(),
                    role,
                    school_name
                ),
                related_entity_id: parsed_id,
                related_entity_type: Some("user".to_string()),
                action_url: None,
            },
            parsed_id,
        ).await;
    }

    pub async fn create_invitation_enhanced(&self, request: CreateInvitationRequestEnhanced) -> ApiResult<CreateInvitationResponse> {
        // Validate email format
        ValidationUtils::validate_email(&request.email)?;

        // Parse and validate school_id - NOW REQUIRED
        let school_uuid = if let Some(ref school_id) = request.school_id {
            ValidationUtils::validate_uuid(school_id)?;
            uuid::Uuid::parse_str(school_id)
                .map_err(|_| AppError::Validation("Invalid school_id format".to_string()))?
        } else {
            // FAIL if school_id is missing for admin/owner invitations
            tracing::warn!("⚠️  No school_id provided for admin/owner invitation");
            return Err(AppError::Validation("school_id is required for admin/owner invitations".to_string()));
        };

        // STEP 1: Fetch school name FIRST - PREREQUISITE VALIDATION
        tracing::info!("🔍 Fetching school name for school_id: {}", school_uuid);

        let school_name = match self.school_dao.get_school_name(&school_uuid).await {
            Ok(name) => {
                tracing::info!("✅ School name fetched: '{}' for school {}", name, school_uuid);
                name  // Return String, not Option<String>
            },
            Err(e) => {
                tracing::error!("❌ Failed to fetch school name for {}: {}", school_uuid, e);
                return Err(AppError::Database(format!(
                    "Cannot create invitation: School name not found for school_id {}: {}",
                    school_uuid, e
                )));
            }
        };

        // STEP 2: Block if email already belongs to a parent in this school
        if self.dao.email_exists_as_parent(&request.email, school_uuid).await? {
            return Err(AppError::Conflict(
                "This email is already registered as a parent. Cannot invite as an admin.".to_string()
            ));
        }

        // Check if user exists (after school validation)
        if self.dao.user_exists_by_email(&request.email).await? {
            return Err(AppError::Conflict("User already exists".to_string()));
        }

        // STEP 3: Create user metadata with VALIDATED school_name
        let metadata = UserMetadata::new(
            Some(school_uuid),
            request.first_name.clone(),
            request.last_name.clone(),
            request.role.clone(),
            None,  // phone_number - not provided in enhanced endpoint
            Some(true),  // is_verified = true
        )
        .with_school_name_option(Some(school_name.clone()));  // school_name is guaranteed to exist

        // STEP 4: Create user in Supabase (no email) then send 7-day branded invite
        let user_id = self.supabase_client.create_user_only_in_supabase(&request.email, metadata).await?;

        let role_str = request.role.as_deref().unwrap_or("Admin");
        let first_name_str = request.first_name.as_deref().unwrap_or("");
        let last_name_str = request.last_name.as_deref().unwrap_or("");
        let invite_token = self.dao
            .create_invite_token(&request.email, role_str, school_uuid)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", request.email, e);
                uuid::Uuid::nil()
            });

        let email_sent = if invite_token != uuid::Uuid::nil() {
            self.supabase_client
                .send_admin_invite_email(&request.email, invite_token, first_name_str, last_name_str, &school_name)
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let email_status = if email_sent { "delivered".to_string() } else { "unknown".to_string() };
        let message = email_status_message(&email_status, "User invitation created successfully. Please check email for confirmation link (valid 7 days).");

        // Suppress for parent invites because the existing parent-invite flow goes
        // through a different path.
        if matches!(role_str, "Admin" | "SuperAdmin" | "Owner") {
            self.fire_admin_added_notification(
                school_uuid,
                &user_id,
                first_name_str,
                last_name_str,
                role_str,
                &school_name,
            ).await;
        }

        Ok(CreateInvitationResponse {
            success: true,
            message,
            email: request.email,
            user_id,
            email_status,
            timestamp: Utc::now(),
        })
    }

    pub async fn create_invitation(&self, request: CreateInvitationRequest) -> ApiResult<CreateInvitationResponse> {
        // Validate email format
        ValidationUtils::validate_email(&request.email)?;

        // Validate and parse school_id (required field)
        ValidationUtils::validate_uuid(&request.school_id)?;
        let school_uuid = uuid::Uuid::parse_str(&request.school_id)
            .map_err(|_| AppError::Validation("Invalid school_id format".to_string()))?;

        // Step 1: Block if email already belongs to a parent in this school
        if self.dao.email_exists_as_parent(&request.email, school_uuid).await? {
            return Err(AppError::Conflict(
                "This email is already registered as a parent. Cannot invite as an admin.".to_string()
            ));
        }

        // Step 2: Active user exists → block with conflict error
        match self.dao.get_user_by_email_and_school(&request.email, school_uuid).await {
            Ok(_) => return Err(AppError::Conflict("Already registered with different role".to_string())),
            Err(AppError::NotFound(_)) => {},
            Err(e) => return Err(e),
        }

        // Step 2: Soft-deleted user exists → reactivate and resend invite
        match self.dao.get_soft_deleted_user_by_email_and_school(&request.email, school_uuid).await {
            Ok(user) => {
                self.dao.reactivate_user(user.id, &request.first_name, &request.last_name).await?;
                self.supabase_client.resend_invitation(&user.email).await?;
                let email_status = self.supabase_client.get_recent_email_status(&user.email).await;
                if matches!(email_status.as_str(), "suppressed" | "bounced") {
                    return Err(AppError::ExternalService(email_status_message(&email_status, "")));
                }
                let message = email_status_message(&email_status, "User reactivated and invitation email resent successfully.");
                return Ok(CreateInvitationResponse {
                    success: true,
                    message,
                    email: user.email,
                    user_id: user.id.to_string(),
                    email_status,
                    timestamp: Utc::now(),
                });
            },
            Err(AppError::NotFound(_)) => {},
            Err(e) => return Err(e),
        }

        // Step 3: New user — create in Supabase then send 7-day branded invite
        let school_name = self.school_dao.get_school_name(&school_uuid).await
            .unwrap_or_else(|_| "Goddard School".to_string());

        let metadata = UserMetadata::new(
            Some(school_uuid),
            Some(request.first_name.clone()),
            Some(request.last_name.clone()),
            Some("Admin".to_string()),
            request.phone_number.clone(),
            Some(true),
        )
        .with_school_name(school_name.clone());

        let user_id = self.supabase_client.create_user_only_in_supabase(&request.email, metadata).await?;

        let invite_token = self.dao
            .create_invite_token(&request.email, "Admin", school_uuid)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", request.email, e);
                uuid::Uuid::nil()
            });

        let email_sent = if invite_token != uuid::Uuid::nil() {
            self.supabase_client
                .send_admin_invite_email(&request.email, invite_token, &request.first_name, &request.last_name, &school_name)
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let email_status = if email_sent { "delivered".to_string() } else { "unknown".to_string() };
        let message = email_status_message(&email_status, "User invitation created successfully. Please check email for confirmation link (valid 7 days).");

        // create_invitation always creates an "Admin" (hardcoded above at create_invite_token),
        // so unconditionally fire — no is_admin_role gate.
        self.fire_admin_added_notification(
            school_uuid,
            &user_id,
            &request.first_name,
            &request.last_name,
            "Admin",
            &school_name,
        ).await;

        Ok(CreateInvitationResponse {
            success: true,
            message,
            email: request.email,
            user_id,
            email_status,
            timestamp: Utc::now(),
        })
    }

    /// Create SuperAdmin user for a school
    /// - Sets role to "SuperAdmin"
    /// - Pre-sets is_verified to true
    /// - Pre-sets is_active to true (via database defaults)
    pub async fn create_superadmin(&self, request: CreateSuperAdminRequest) -> ApiResult<CreateInvitationResponse> {
        // Step 1: Validate email format
        ValidationUtils::validate_email(&request.email)?;

        // Step 2: Validate and parse school_id (required)
        ValidationUtils::validate_uuid(&request.school_id)?;
        let school_uuid = uuid::Uuid::parse_str(&request.school_id)
            .map_err(|_| AppError::Validation("Invalid school_id format".to_string()))?;

        // Step 3: Block if email already belongs to a parent in this school
        if self.dao.email_exists_as_parent(&request.email, school_uuid).await? {
            return Err(AppError::Conflict(
                "This email is already registered as a parent. Cannot invite as an admin.".to_string()
            ));
        }

        // Check if user already exists
        if self.dao.user_exists_by_email(&request.email).await? {
            return Err(AppError::Conflict("User already exists".to_string()));
        }

        // Step 4: Look up school name for invite email
        let school_name = self.school_dao.get_school_name(&school_uuid).await
            .unwrap_or_else(|_| "Goddard School".to_string());

        // Step 5: Build metadata with role = "SuperAdmin" and is_verified = true
        let metadata = UserMetadata::new(
            Some(school_uuid),
            Some(request.first_name.clone()),
            Some(request.last_name.clone()),
            Some("SuperAdmin".to_string()),  // ROLE = SuperAdmin
            request.phone_number.clone(),
            Some(true),  // is_verified = true
        )
        .with_school_name(school_name.clone());

        // Step 6: Create user in Supabase then send 7-day branded invite
        let user_id = self.supabase_client
            .create_user_only_in_supabase(&request.email, metadata)
            .await?;

        let invite_token = self.dao
            .create_invite_token(&request.email, "SuperAdmin", school_uuid)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", request.email, e);
                uuid::Uuid::nil()
            });

        let email_sent = if invite_token != uuid::Uuid::nil() {
            self.supabase_client
                .send_admin_invite_email(&request.email, invite_token, &request.first_name, &request.last_name, &school_name)
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let email_status = if email_sent { "delivered".to_string() } else { "unknown".to_string() };
        let message = email_status_message(&email_status, "SuperAdmin user created successfully. Please check email for confirmation link (valid 7 days).");

        Ok(CreateInvitationResponse {
            success: true,
            message,
            email: request.email,
            user_id,
            email_status,
            timestamp: Utc::now(),
        })
    }

    pub async fn clear_auth_table(&self) -> ApiResult<()> {
        self.supabase_client.clear_auth_table().await?;
        Ok(())
    }

    pub async fn debug_list_auth_users(&self) -> ApiResult<serde_json::Value> {
        let users = self.supabase_client.debug_list_all_users().await?;
        Ok(users)
    }

    pub async fn get_users_by_school_and_role(&self, school_id: &str, role: &str) -> ApiResult<Vec<FilteredUserResponse>> {
        // Validate inputs
        ValidationUtils::validate_uuid(school_id)?;

        // Get filtered users from Supabase auth
        let supabase_users = self.supabase_client.get_users_by_school_and_role(school_id, role).await?;

        let mut response_users = Vec::new();

        for user in supabase_users {
            // Extract user data from Supabase auth response
            let user_id = user.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let email = user.get("email").and_then(|v| v.as_str()).unwrap_or_default();

            let metadata = user.get("user_metadata");
            let first_name = metadata
                .and_then(|m| m.get("first_name"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let last_name = metadata
                .and_then(|m| m.get("last_name"))
                .and_then(|v| v.as_str())
                .map(String::from);

            // For parent role, use user_id as both user_id and parent_id
            let parent_id = if role.to_lowercase() == "parent" {
                Some(user_id.to_string())
            } else {
                None
            };

            response_users.push(FilteredUserResponse {
                school_id: school_id.to_string(),
                role: role.to_string(),
                user_id: user_id.to_string(),
                email: email.to_string(),
                parent_id,
                first_name,
                last_name,
                school_data: None,
            });
        }

        Ok(response_users)
    }

    /// Update admin user - Admin can only update THEIR OWN profile, SuperAdmin can update ANY admin
    /// User ID is extracted from JWT token (AuthContext), or from payload for SuperAdmin
    pub async fn update_admin_user(
        &self,
        auth_user_id: uuid::Uuid,  // From AuthContext (JWT)
        auth_role: crate::models::schema::UserRole,  // From AuthContext (JWT)
        request: UpdateAdminRequest,
    ) -> ApiResult<AdminUserResponse> {
        // Determine which user to update based on role
        let target_user_id = if matches!(auth_role, crate::models::schema::UserRole::SuperAdmin) {
            // SuperAdmin can update any admin if user_id provided in payload
            if let Some(ref uid) = request.user_id {
                ValidationUtils::validate_uuid(uid)?;
                uuid::Uuid::parse_str(uid)
                    .map_err(|_| AppError::Validation("Invalid user_id format".to_string()))?
            } else {
                auth_user_id  // Update own profile if no user_id provided
            }
        } else {
            // Admin can only update their own profile (ignore user_id in payload)
            auth_user_id
        };

        let updated = self.dao.update_admin_user(
            target_user_id,
            request.first_name,
            request.last_name,
            request.phone_number,
        ).await?;

        Ok(AdminUserResponse {
            id: updated.id.to_string(),
            school_id: updated.school_id.to_string(),
            first_name: updated.first_name,
            last_name: updated.last_name,
            email: updated.email,
            role: updated.role,
            is_verified: updated.is_verified,
        })
    }

    /// Soft delete admin user (SuperAdmin only)
    pub async fn delete_admin_user(&self, request: DeleteAdminRequest) -> ApiResult<()> {
        ValidationUtils::validate_uuid(&request.user_id)?;
        let user_uuid = uuid::Uuid::parse_str(&request.user_id)
            .map_err(|_| AppError::Validation("Invalid user_id format".to_string()))?;

        // Capture identity BEFORE the soft delete — the post-delete row has
        // is_active=false, and several lookups filter that out, so we'd lose
        // the name/email needed to render a useful notification body.
        let admin_details = self.dao.get_user_by_id(user_uuid).await.ok();

        self.dao.soft_delete_admin_user(user_uuid).await?;

        if let Some(d) = admin_details {
            self.notification_service.notify_school_admins(
                crate::models::notification::CreateNotification {
                    school_id: d.school_id,
                    notification_type: crate::models::notification::notification_type::ADMIN_DEACTIVATED.to_string(),
                    title: "Admin Deactivated".to_string(),
                    body: format!(
                        "{} {} ({}) has been deactivated as {}.",
                        d.first_name.trim(),
                        d.last_name.trim(),
                        d.email,
                        d.role
                    ),
                    related_entity_id: Some(d.id),
                    related_entity_type: Some("user".to_string()),
                    action_url: None,
                },
                Some(user_uuid),
            ).await;
        }

        Ok(())
    }

    /// Get all verified Admin users for a specific school (SuperAdmin only)
    pub async fn get_admins_by_school(&self, school_id: &str) -> ApiResult<Vec<AdminUserResponse>> {
        ValidationUtils::validate_uuid(school_id)?;
        let school_uuid = uuid::Uuid::parse_str(school_id)
            .map_err(|_| AppError::Validation("Invalid school_id format".to_string()))?;

        let admins = self.dao.get_admins_by_school(school_uuid).await?;

        Ok(admins.into_iter().map(|u| AdminUserResponse {
            id: u.id.to_string(),
            school_id: u.school_id.to_string(),
            first_name: u.first_name,
            last_name: u.last_name,
            email: u.email,
            role: u.role,
            is_verified: u.is_verified,
        }).collect())
    }

    pub async fn get_user_profile_from_jwt(&self, jwt_token: &str) -> ApiResult<FilteredUserResponse> {
        // Verify JWT to get user ID only (don't need full user data from auth table)
        let user_data = self.supabase_client.verify_jwt_and_get_user(jwt_token).await?;

        // Extract user ID from JWT
        let user_id_str = user_data.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ExternalService("Missing user ID in JWT response".to_string()))?;

        let user_id = uuid::Uuid::parse_str(user_id_str)
            .map_err(|_| AppError::Validation("Invalid user ID format".to_string()))?;

        // Query our users table directly for user information
        let user = self.dao.get_user_by_id(user_id).await?;

        // Check if user is verified
        if !user.is_verified {
            return Err(AppError::Authorization("User verification failed. Please verify your account.".to_string()));
        }

        // For parent role, use user_id as both user_id and parent_id
        let parent_id = if user.role.to_lowercase() == "parent" {
            Some(user.id.to_string())
        } else {
            None
        };

        let school_data = match self.school_dao.get_school_by_id(&user.school_id).await {
            Ok(Some(school)) => Some(SchoolResponse {
                id: school.id,
                name: school.name,
                subdomain: school.subdomain,
                settings: school.settings,
                created_at: school.created_at,
                updated_at: school.updated_at,
            }),
            _ => None,
        };

        Ok(FilteredUserResponse {
            school_id: user.school_id.to_string(),
            role: user.role,
            user_id: user.id.to_string(),
            email: user.email,
            parent_id,
            first_name: Some(user.first_name),
            last_name: Some(user.last_name),
            school_data,
        })
    }

    pub async fn forgot_password(&self, request: ForgotPasswordRequest) -> ApiResult<ForgotPasswordResponse> {
        // Verify email exists in the system (no school filter)
        self.dao.get_user_by_email(&request.email).await?;

        // User exists — trigger password reset email via Supabase
        self.supabase_client.send_password_reset_email(&request.email).await?;

        Ok(ForgotPasswordResponse {
            success: true,
            message: "Password reset email sent. Please check your inbox.".to_string(),
            email: request.email,
        })
    }

    pub async fn resend_admin_invite(&self, request: ResendAdminInviteRequest) -> ApiResult<ResendAdminInviteResponse> {
        // Step 1: Look up admin in users table (has first_name, last_name, school_id)
        let user = self.dao.get_user_by_id(request.user_id).await?;

        // Step 2: Role check
        if user.role.to_lowercase() != "admin" {
            return Err(AppError::Validation("User is not an Admin".to_string()));
        }

        // Step 3: Get school name for email template
        let school_name = self.school_dao.get_school_name(&user.school_id).await
            .unwrap_or_else(|_| "Goddard School".to_string());

        // Step 4: Create a fresh 7-day invite token
        let invite_token = self.dao
            .create_invite_token(&user.email, &user.role, user.school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", user.email, e);
                uuid::Uuid::nil()
            });

        // Step 5: Send branded admin invite email via Resend
        let email_sent = if invite_token != uuid::Uuid::nil() {
            self.supabase_client
                .send_admin_invite_email(&user.email, invite_token, &user.first_name, &user.last_name, &school_name)
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let email_status = self.supabase_client.get_recent_email_status(&user.email).await;
        if matches!(email_status.as_str(), "suppressed" | "bounced") {
            return Err(AppError::ExternalService(email_status_message(&email_status, "")));
        }

        let message = email_status_message(&email_status, "Admin invitation email resent successfully.");

        Ok(ResendAdminInviteResponse {
            user_id: user.id.to_string(),
            email: user.email,
            email_sent,
            message,
            email_status,
        })
    }
}