use crate::{
    dao::AuthDao,
    error::{AppError, ApiResult},
    utils::ValidationUtils,
    services::{SupabaseClient, supabase_client::UserMetadata},
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
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub school_id: Option<String>,
    pub user_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequestEnhanced {
    pub email: String,
    pub school_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateInvitationResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
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

    pub async fn create_invitation_enhanced(&self, request: CreateInvitationRequestEnhanced) -> ApiResult<CreateInvitationResponse> {
        // Validate email format
        ValidationUtils::validate_email(&request.email)?;

        // Parse and validate school_id if provided
        let school_uuid = if let Some(ref school_id) = request.school_id {
            ValidationUtils::validate_uuid(school_id)?;
            Some(uuid::Uuid::parse_str(school_id)
                .map_err(|_| AppError::Validation("Invalid school_id format".to_string()))?)
        } else {
            None
        };

        // Check if user already exists
        if self.dao.user_exists_by_email(&request.email).await? {
            return Err(AppError::Conflict("User already exists".to_string()));
        }

        // Create user metadata
        let metadata = UserMetadata::new(
            school_uuid,
            request.first_name.clone(),
            request.last_name.clone(),
            request.role.clone(),
        );

        // Create user invitation via Supabase with enhanced metadata
        let user_id = self.supabase_client.create_user_invitation_enhanced(&request.email, metadata).await?;

        Ok(CreateInvitationResponse {
            success: true,
            message: "User invitation created successfully with custom fields. Please check email for confirmation link.".to_string(),
            email: request.email,
            user_id,
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
            });
        }

        Ok(response_users)
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

        Ok(FilteredUserResponse {
            school_id: user.school_id.to_string(),
            role: user.role,
            user_id: user.id.to_string(),
            email: user.email,
            parent_id,
            first_name: Some(user.first_name),
            last_name: Some(user.last_name),
        })
    }
}