use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::dao::auth_dao::AuthDao;
use crate::dao::owner_dao::OwnerDao;
use crate::dao::school_dao::SchoolDao;
use crate::error::{AppError, ApiResult};
use crate::middleware::auth::AuthContext;
use crate::models::schema::UserRole;
use crate::services::supabase_client::{SupabaseClient, UserMetadata};
use crate::utils::ValidationUtils;

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InviteOwnerRequest {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOwnerRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResendOwnerInviteRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct OwnerResponse {
    pub id: String,
    pub school_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,
    pub is_verified: bool,
    pub phone_number: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuperAdminInfo {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone_number: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OwnersListResponse {
    pub success: bool,
    pub count: usize,
    pub data: Vec<OwnerResponse>,
    pub super_admin: Option<SuperAdminInfo>,
}

#[derive(Debug, Serialize)]
pub struct InviteOwnerResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
    pub user_id: String,
    pub email_status: String,
}

#[derive(Debug, Serialize)]
pub struct ResendOwnerInviteResponse {
    pub user_id: String,
    pub email: String,
    pub email_sent: bool,
    pub message: String,
}

// ── Service ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct OwnerService {
    owner_dao: OwnerDao,
    auth_dao: AuthDao,
    school_dao: SchoolDao,
    supabase_client: SupabaseClient,
}

impl OwnerService {
    pub fn new(
        owner_dao: OwnerDao,
        auth_dao: AuthDao,
        school_dao: SchoolDao,
        supabase_client: SupabaseClient,
    ) -> Self {
        Self { owner_dao, auth_dao, school_dao, supabase_client }
    }

    pub async fn invite_owner(
        &self,
        request: InviteOwnerRequest,
        auth: &AuthContext,
    ) -> ApiResult<InviteOwnerResponse> {
        ValidationUtils::validate_email(&request.email)?;

        let school_id = auth.school_id;

        // Block if email already belongs to a parent in this school
        if self.auth_dao.email_exists_as_parent(&request.email, school_id).await? {
            return Err(AppError::Conflict(
                "This email is already registered as a parent. Cannot invite as an Owner.".to_string()
            ));
        }

        // Block if email already exists as any user
        if self.auth_dao.user_exists_by_email(&request.email).await? {
            return Err(AppError::Conflict("A user with this email already exists.".to_string()));
        }

        let school_name = self.school_dao.get_school_name(&school_id).await
            .unwrap_or_else(|_| "Goddard School".to_string());

        let metadata = UserMetadata::new(
            Some(school_id),
            Some(request.first_name.clone()),
            Some(request.last_name.clone()),
            Some("Owner".to_string()),
            request.phone_number.clone(),
            Some(true), // is_verified = true
        )
        .with_school_name(school_name.clone());

        let user_id = self.supabase_client
            .create_user_only_in_supabase(&request.email, metadata)
            .await?;

        let invite_token = self.auth_dao
            .create_invite_token(&request.email, "Owner", school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", request.email, e);
                Uuid::nil()
            });

        let email_sent = if invite_token != Uuid::nil() {
            self.supabase_client
                .send_admin_invite_email(
                    &request.email,
                    invite_token,
                    &request.first_name,
                    &request.last_name,
                    &school_name,
                    "Owner",
                )
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let email_status = if email_sent { "delivered".to_string() } else { "unknown".to_string() };

        Ok(InviteOwnerResponse {
            success: true,
            message: format!("Owner invitation sent. Please check email for confirmation link (valid 7 days)."),
            email: request.email,
            user_id,
            email_status,
        })
    }

    pub async fn list_owners(&self, auth: &AuthContext) -> ApiResult<OwnersListResponse> {
        let owners = self.owner_dao.get_owners_by_school(auth.school_id).await?;

        let super_admin = self.auth_dao
            .get_superadmin_with_login_status_by_school(&auth.school_id)
            .await
            .unwrap_or(None)
            .map(|sa| SuperAdminInfo {
                id: sa.id.to_string(),
                first_name: sa.first_name,
                last_name: sa.last_name,
                email: sa.email,
                phone_number: sa.phone_number,
            });

        let data: Vec<OwnerResponse> = owners.into_iter().map(|u| OwnerResponse {
            id: u.id.to_string(),
            school_id: u.school_id.to_string(),
            first_name: u.first_name,
            last_name: u.last_name,
            email: u.email,
            role: u.role,
            is_verified: u.is_verified,
            phone_number: u.phone_number,
        }).collect();

        let count = data.len();
        Ok(OwnersListResponse { success: true, count, data, super_admin })
    }

    pub async fn update_owner(
        &self,
        id: Uuid,
        request: UpdateOwnerRequest,
        auth: &AuthContext,
    ) -> ApiResult<OwnerResponse> {
        let updated = self.owner_dao.update_owner(
            id,
            auth.school_id,
            request.first_name,
            request.last_name,
            request.phone_number,
        ).await?;

        Ok(OwnerResponse {
            id: updated.id.to_string(),
            school_id: updated.school_id.to_string(),
            first_name: updated.first_name,
            last_name: updated.last_name,
            email: updated.email,
            role: updated.role,
            is_verified: updated.is_verified,
            phone_number: updated.phone_number,
        })
    }

    pub async fn delete_owner(&self, id: Uuid, auth: &AuthContext) -> ApiResult<()> {
        // Prevent self-deletion
        if id == auth.user_id {
            return Err(AppError::Validation("Cannot deactivate your own Owner account.".to_string()));
        }
        self.owner_dao.soft_delete_owner(id, auth.school_id).await
    }

    pub async fn resend_invite(
        &self,
        request: ResendOwnerInviteRequest,
        auth: &AuthContext,
    ) -> ApiResult<ResendOwnerInviteResponse> {
        ValidationUtils::validate_uuid(&request.user_id)?;
        let user_uuid = Uuid::parse_str(&request.user_id)
            .map_err(|_| AppError::Validation("Invalid user_id format".to_string()))?;

        let owner = self.owner_dao
            .get_owner_by_id(user_uuid, auth.school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Owner not found".to_string()))?;

        let school_name = self.school_dao.get_school_name(&auth.school_id).await
            .unwrap_or_else(|_| "Goddard School".to_string());

        let invite_token = self.auth_dao
            .create_invite_token(&owner.email, "Owner", auth.school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", owner.email, e);
                Uuid::nil()
            });

        let email_sent = if invite_token != Uuid::nil() {
            self.supabase_client
                .send_admin_invite_email(
                    &owner.email,
                    invite_token,
                    &owner.first_name,
                    &owner.last_name,
                    &school_name,
                    "Owner",
                )
                .await
                .unwrap_or(false)
        } else {
            false
        };

        Ok(ResendOwnerInviteResponse {
            user_id: owner.id.to_string(),
            email: owner.email,
            email_sent,
            message: "Owner invitation email resent successfully.".to_string(),
        })
    }
}
