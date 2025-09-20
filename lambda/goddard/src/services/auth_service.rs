use crate::{
    dao::AuthDao,
    error::{AppError, ApiResult},
    utils::ValidationUtils,
    services::SupabaseClient,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub school_id: Option<String>,
    pub user_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateInvitationResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
}

pub struct AuthService {
    dao: AuthDao,
    supabase_client: SupabaseClient,
}

impl AuthService {
    pub fn new(dao: AuthDao, supabase_client: SupabaseClient) -> Self {
        Self { dao, supabase_client }
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
                // Successfully sent email, update timestamp
                self.dao.update_confirmation_sent_at(&request.email).await?;
            }
            Err(AppError::ExternalService(msg)) if msg.contains("rate limit") => {
                // Rate limited - just update timestamp since user already exists
                // and they know about the previous email
                self.dao.update_confirmation_sent_at(&request.email).await?;
            }
            Err(e) => return Err(e),
        }

        Ok(ResendInvitationResponse {
            success: true,
            message: "Invitation processed successfully. If no email was received, please check spam folder or wait 60 seconds before requesting again.".to_string(),
            email: request.email,
            timestamp: Utc::now(),
        })
    }

    pub async fn create_invitation(&self, request: CreateInvitationRequest) -> ApiResult<CreateInvitationResponse> {
        // Validate email format
        ValidationUtils::validate_email(&request.email)?;

        // Validate school_id if provided
        if let Some(ref school_id) = request.school_id {
            ValidationUtils::validate_uuid(school_id)?;
        }

        // Check if user already exists
        if self.dao.user_exists_by_email(&request.email).await? {
            return Err(AppError::Conflict("User already exists".to_string()));
        }

        // Create user invitation via Supabase
        let user_id = self.supabase_client.create_user_invitation(&request.email, request.user_metadata).await?;

        Ok(CreateInvitationResponse {
            success: true,
            message: "User invitation created successfully. Please check email for confirmation link.".to_string(),
            email: request.email,
            user_id,
            timestamp: Utc::now(),
        })
    }

    pub async fn clear_auth_table(&self) -> ApiResult<()> {
        self.supabase_client.clear_auth_table().await?;
        Ok(())
    }
}