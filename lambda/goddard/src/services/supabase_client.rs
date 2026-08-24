use crate::error::AppError;
use crate::services::email_service::EmailService;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMetadata {
    #[serde(serialize_with = "serialize_uuid_option", deserialize_with = "deserialize_uuid_option")]
    pub school_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
    pub phone_number: Option<String>,
    pub is_verified: Option<bool>,
    pub school_name: Option<String>,  // NEW: For email template personalization
}

fn serialize_uuid_option<S>(uuid: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match uuid {
        Some(u) => serializer.serialize_str(&u.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_uuid_option<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => Uuid::parse_str(&s).map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

impl UserMetadata {
    pub fn new(
        school_id: Option<Uuid>,
        first_name: Option<String>,
        last_name: Option<String>,
        role: Option<String>,
        phone_number: Option<String>,
        is_verified: Option<bool>,
    ) -> Self {
        Self {
            school_id,
            first_name,
            last_name,
            role,
            phone_number,
            is_verified,
            school_name: None,  // Default to None
        }
    }

    /// Builder method to set school_name for email personalization
    pub fn with_school_name(mut self, school_name: String) -> Self {
        self.school_name = Some(school_name);
        self
    }

    /// Builder method to set school_name from Option<String>
    pub fn with_school_name_option(mut self, school_name: Option<String>) -> Self {
        self.school_name = school_name;
        self
    }

    /// Convert to Supabase-compatible metadata format
    /// Fields are placed at the top level for:
    /// 1. Database trigger to extract values (e.g., raw_user_meta_data->>'first_name')
    /// 2. Email templates to access via {{ .Data.first_name }} (where .Data is the entire metadata object)
    pub fn to_supabase_metadata(&self) -> serde_json::Value {
        json!({
            "school_id": self.school_id.map(|id| id.to_string()),
            "role": self.role,
            "is_verified": self.is_verified,
            "school_name": self.school_name,
            "first_name": self.first_name,
            "last_name": self.last_name,
            "phone_number": self.phone_number
        })
    }
}

#[derive(Clone)]
pub struct SupabaseClient {
    client: Client,
    project_url: String,
    service_role_key: String,
    anon_key: String,
    frontend_url: String,
    api_base_url: String,
    email_service: Arc<EmailService>,
}

impl SupabaseClient {
    pub fn new(email_service: Arc<EmailService>) -> Result<Self, AppError> {
        let project_url = env::var("SUPABASE_URL")
            .map_err(|_| AppError::Internal("SUPABASE_URL must be set".to_string()))?;
        let service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| AppError::Internal("SUPABASE_SERVICE_ROLE_KEY must be set".to_string()))?;
        let anon_key = env::var("SUPABASE_ANON_KEY")
            .map_err(|_| AppError::Internal("SUPABASE_ANON_KEY must be set".to_string()))?;

        // Check if the service role key is still the placeholder
        if service_role_key == "your_actual_service_role_key_here" {
            return Err(AppError::Internal(
                "SUPABASE_SERVICE_ROLE_KEY is still set to placeholder. Please replace with actual service role key from Supabase dashboard.".to_string()
            ));
        }

        // Read frontend URL from environment, with fallback to dev URL
        let frontend_url = env::var("FRONTEND_URL")
            .unwrap_or_else(|_| {
                eprintln!("⚠️  FRONTEND_URL not set, using default dev URL");
                "https://dev.goddard-web.pages.dev".to_string()
            });

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let api_base_url = env::var("API_BASE_URL")
            .unwrap_or_else(|_| "https://api.goddard-app.com".to_string());

        Ok(Self {
            client,
            project_url,
            service_role_key,
            anon_key,
            frontend_url,
            api_base_url,
            email_service,
        })
    }

    pub async fn resend_invitation(&self, email: &str) -> Result<(), AppError> {
        tracing::info!("Resending invitation/confirmation to: {}", email);

        // Step 1: Get user by email to check confirmation status
        let user_result = self.get_user_by_email(email).await;

        // Step 2: Determine email type based on user confirmation status
        let (endpoint, body, email_type) = match user_result {
            Ok(user) => {
                // Check if email is confirmed
                let is_confirmed = user
                    .get("email_confirmed_at")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);

                if is_confirmed {
                    // For confirmed users, send magic link for one-time passwordless sign-in
                    tracing::info!("User already confirmed, sending magic link email");
                    (
                        format!("{}/auth/v1/magiclink", self.project_url),
                        json!({
                            "email": email,
                            "options": {
                                "emailRedirectTo": format!("{}/auth/callback", self.frontend_url)
                            }
                        }),
                        "magic_link"
                    )
                } else {
                    // For unconfirmed users, resend signup confirmation
                    tracing::info!("User not confirmed, sending signup confirmation email");
                    (
                        format!("{}/auth/v1/resend", self.project_url),
                        json!({
                            "type": "signup",
                            "email": email
                        }),
                        "signup_confirmation"
                    )
                }
            }
            Err(_) => {
                // User not found - send signup confirmation as fallback
                tracing::warn!("User not found, sending signup confirmation email");
                (
                    format!("{}/auth/v1/resend", self.project_url),
                    json!({
                        "type": "signup",
                        "email": email
                    }),
                    "signup_confirmation"
                )
            }
        };

        tracing::info!("Sending {} email to {}", email_type, email);

        // Step 3: Send email
        let invite_response = self.client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send {}: {}", email_type, e)))?;

        if !invite_response.status().is_success() {
            let status_code = invite_response.status();
            let error_text = invite_response.text().await.unwrap_or_default();

            // Handle rate limiting with user-friendly message
            if status_code == 429 || error_text.contains("over_email_send_rate_limit") {
                return Err(AppError::ExternalService(
                    "Email rate limit exceeded. Please wait 60 seconds before sending another invitation to the same email address.".to_string()
                ));
            }

            return Err(AppError::ExternalService(format!("Failed to send {}: {}", email_type, error_text)));
        }

        tracing::info!("✅ {} email sent successfully to {}", email_type, email);
        Ok(())
    }

    /// Get user by email address
    pub async fn get_user_by_email(&self, email: &str) -> Result<serde_json::Value, AppError> {
        // List all users and filter by email (Supabase doesn't support direct email lookup)
        let response = self.client
            .get(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to list users: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("Failed to list users: {}", error_text)));
        }

        let users_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse users list: {}", e)))?;

        let empty_vec = vec![];
        let users = users_response
            .get("users")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        // Find user with matching email
        users
            .iter()
            .find(|user| {
                user.get("email")
                    .and_then(|e| e.as_str())
                    .map(|e| e.eq_ignore_ascii_case(email))
                    .unwrap_or(false)
            })
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("User with email {} not found", email)))
    }

    /// Creates a new user and sends "Confirm Sign Up" email template
    ///
    /// This method:
    /// 1. Creates user via /auth/v1/admin/users with email_confirm=false
    /// 2. Sends "Confirm Sign Up" email via /auth/v1/resend (type: "signup")
    ///
    /// Use this for parent invitations where users need to confirm their email.
    pub async fn create_user_with_signup_confirmation(
        &self,
        email: &str,
        metadata: UserMetadata,
    ) -> Result<(String, bool), AppError> {
        tracing::info!("Creating user with signup confirmation for {}", email);

        // Step 1: Create user via Admin API without email confirmation
        let user_metadata_json = metadata.to_supabase_metadata();

        let create_user_body = json!({
            "email": email,
            "email_confirm": false,  // User must confirm email
            "user_metadata": user_metadata_json,
            "app_metadata": {
                "provider": "email",
                "providers": ["email"]
            }
        });

        tracing::debug!("Creating user with body: {}",
            serde_json::to_string_pretty(&create_user_body).unwrap_or_default());

        let create_response = self.client
            .post(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&create_user_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to create user: {}", e)))?;

        // Handle response
        let status_code = create_response.status();

        if !status_code.is_success() {
            let error_body = create_response.text().await.unwrap_or_default();
            tracing::error!("User creation failed with status {}: {}", status_code, error_body);

            // Handle specific error cases
            if status_code == 422 && error_body.contains("already been registered") {
                return Err(AppError::Conflict("User with this email already exists".to_string()));
            }

            return Err(AppError::ExternalService(format!("Failed to create user: {}", error_body)));
        }

        let user_data: serde_json::Value = create_response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user creation response: {}", e)))?;

        // Extract user ID
        let user_id = user_data
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| AppError::ExternalService("User ID not found in response".to_string()))?;

        tracing::info!("User created successfully with ID: {}", user_id);

        // Step 2: Send "Confirm Sign Up" email via resend endpoint (non-fatal)
        tracing::info!("Sending confirmation email to {}", email);
        let email_sent = match self.resend_invitation(email).await {
            Ok(()) => {
                tracing::info!("Confirmation email sent to {}", email);
                true
            }
            Err(e) => {
                tracing::warn!("User created but confirmation email failed for {}: {}. Email can be resent later.", email, e);
                false
            }
        };

        Ok((user_id.to_string(), email_sent))
    }

    pub async fn create_user_invitation_enhanced(&self, email: &str, metadata: UserMetadata) -> Result<String, AppError> {
        // Use "signup" for all roles - Supabase only supports: signup, email_change, sms, phone_change
        let template_type = "signup";  // ✅ VALID for all user types

        tracing::info!("[SupabaseClient] Sending email to {} with template type: {}", email, template_type);

        // Log the actual metadata values BEFORE transformation
        tracing::info!("📋 [SupabaseClient] Metadata before transformation: first_name={:?}, last_name={:?}, school_name={:?}",
            metadata.first_name, metadata.last_name, metadata.school_name);

        // Convert to Supabase-compatible metadata format
        let user_metadata = metadata.to_supabase_metadata();

        // Log the transformed metadata JSON
        tracing::info!("📋 [SupabaseClient] Transformed metadata JSON: {}",
            serde_json::to_string_pretty(&user_metadata).unwrap_or_default());

        // Specifically log school_name value
        if let Some(school_name) = user_metadata.get("school_name") {
            tracing::info!("📋 [SupabaseClient] school_name in metadata: {:?}", school_name);
        } else {
            tracing::warn!("⚠️  [SupabaseClient] school_name is MISSING from metadata!");
        }

        self.create_user_invitation_with_template(email, Some(user_metadata), template_type).await
    }

    pub async fn create_user_invitation_with_template(&self, email: &str, user_metadata: Option<serde_json::Value>, _template_type: &str) -> Result<String, AppError> {
        // Note: template_type parameter is kept for backward compatibility but unused
        // The /auth/v1/invite endpoint always uses the "Invite User" template

        // Build invite request body
        let mut invite_request_body = json!({
            "email": email,
        });

        // Add user metadata if provided
        // Note: The key is "data" not "user_metadata" for the invite endpoint
        if let Some(metadata) = user_metadata {
            invite_request_body["data"] = metadata;
        }

        // Add redirect URL for password setup
        invite_request_body["options"] = json!({
            "emailRedirectTo": format!("{}/set-password", self.frontend_url)
        });

        tracing::info!("📧 Sending invitation email using /auth/v1/invite endpoint");
        tracing::debug!("📧 Invite request body: {}",
            serde_json::to_string_pretty(&invite_request_body).unwrap_or_default());

        // Send invitation (creates user + sends "Invite User" template in one request)
        let invite_response = self.client
            .post(&format!("{}/auth/v1/invite", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&invite_request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send invitation: {}", e)))?;

        let status_code = invite_response.status();
        tracing::info!("📧 Invite response status: {}", status_code);

        if !status_code.is_success() {
            let error_body = invite_response.text().await.unwrap_or_default();
            tracing::error!("📧 Invite failed with status {}: {}", status_code, error_body);

            // Handle specific error cases
            if status_code == 422 && error_body.contains("already been registered") {
                return Err(AppError::Conflict("User with this email already exists".to_string()));
            }

            return Err(AppError::ExternalService(format!("Failed to send invitation: {}", error_body)));
        }

        let invite_data: serde_json::Value = invite_response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse invite response: {}", e)))?;

        tracing::info!("✅ Invitation sent successfully to {}", email);

        // Extract user ID from response
        let user_id = invite_data
            .get("user")
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_str())
            .or_else(|| invite_data.get("id").and_then(|id| id.as_str()))
            .ok_or_else(|| AppError::ExternalService(
                format!("User ID not found in invite response. Response: {}",
                    serde_json::to_string(&invite_data).unwrap_or_default())
            ))?;

        Ok(user_id.to_string())
    }

    /// Legacy method for backwards compatibility
    pub async fn create_user_invitation(&self, email: &str, user_metadata: Option<serde_json::Value>) -> Result<String, AppError> {
        // Default to "signup" template for backwards compatibility
        self.create_user_invitation_with_template(email, user_metadata, "signup").await
    }

    pub async fn create_auth_user(&self, email: &str) -> Result<uuid::Uuid, AppError> {
        // Create user in Supabase Auth and return the auth user ID
        let user_id_string = self.create_user_invitation(email, None).await?;

        // Parse the user ID string into UUID
        let auth_user_id = uuid::Uuid::parse_str(&user_id_string)
            .map_err(|e| AppError::ExternalService(format!("Invalid user ID format from Supabase: {}", e)))?;

        Ok(auth_user_id)
    }

    pub async fn get_user_email_by_id(&self, user_id: uuid::Uuid) -> Result<String, AppError> {
        let user_id_str = user_id.to_string();

        let response = self.client
            .get(&format!("{}/auth/v1/admin/users/{}", self.project_url, user_id_str))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to get user from Supabase: {}", e)))?;

        if !response.status().is_success() {
            let status_code = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status_code == 404 {
                return Err(AppError::NotFound("User not found in Supabase auth".to_string()));
            }

            return Err(AppError::ExternalService(format!(
                "Failed to get user from Supabase. Status: {}, Error: {}",
                status_code, error_text
            )));
        }

        let user_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user response: {}", e)))?;

        let email = user_data
            .get("email")
            .and_then(|email| email.as_str())
            .ok_or_else(|| AppError::ExternalService("Email not found in user response".to_string()))?;

        Ok(email.to_string())
    }

    pub async fn get_user_auth_details(&self, user_id: uuid::Uuid) -> Result<(String, DateTime<Utc>, bool), AppError> {
        let user_id_str = user_id.to_string();

        let response = self.client
            .get(&format!("{}/auth/v1/admin/users/{}", self.project_url, user_id_str))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to get user from Supabase: {}", e)))?;

        if !response.status().is_success() {
            let status_code = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status_code == 404 {
                return Err(AppError::NotFound("User not found in Supabase auth".to_string()));
            }

            return Err(AppError::ExternalService(format!(
                "Failed to get user from Supabase. Status: {}, Error: {}",
                status_code, error_text
            )));
        }

        let user_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user response: {}", e)))?;

        let email = user_data
            .get("email")
            .and_then(|email| email.as_str())
            .ok_or_else(|| AppError::ExternalService("Email not found in user response".to_string()))?;

        let created_at_str = user_data
            .get("created_at")
            .and_then(|created_at| created_at.as_str())
            .ok_or_else(|| AppError::ExternalService("Created_at not found in user response".to_string()))?;

        let created_at = created_at_str.parse::<DateTime<Utc>>()
            .map_err(|e| AppError::ExternalService(format!("Failed to parse created_at: {}", e)))?;

        let last_sign_in_at = user_data
            .get("last_sign_in_at")
            .and_then(|value| value.as_str());

        let id_signed = last_sign_in_at.is_some() && !last_sign_in_at.unwrap().is_empty();

        Ok((email.to_string(), created_at, id_signed))
    }

    pub async fn get_user_metadata(&self, user_id: uuid::Uuid) -> Result<Option<UserMetadata>, AppError> {
        let user_id_str = user_id.to_string();

        let response = self.client
            .get(&format!("{}/auth/v1/admin/users/{}", self.project_url, user_id_str))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to get user from Supabase: {}", e)))?;

        if !response.status().is_success() {
            let status_code = response.status();
            if status_code == 404 {
                return Ok(None);
            }
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Failed to get user from Supabase. Status: {}, Error: {}",
                status_code, error_text
            )));
        }

        let user_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user response: {}", e)))?;

        // Extract user_metadata
        if let Some(metadata_value) = user_data.get("user_metadata") {
            let user_metadata: UserMetadata = serde_json::from_value(metadata_value.clone())
                .unwrap_or_else(|_| UserMetadata::new(None, None, None, None, None, None));
            Ok(Some(user_metadata))
        } else {
            Ok(None)
        }
    }

    pub async fn clear_auth_table(&self) -> Result<(), AppError> {
        // First, get all users
        let list_response = self.client
            .get(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to list users: {}", e)))?;

        if !list_response.status().is_success() {
            let error_text = list_response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("Failed to list users: {}", error_text)));
        }

        let users: serde_json::Value = list_response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse users list: {}", e)))?;

        // Extract user IDs
        let user_ids: Vec<String> = if let Some(users_array) = users.get("users").and_then(|v| v.as_array()) {
            users_array
                .iter()
                .filter_map(|user| user.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect()
        } else if let Some(users_array) = users.as_array() {
            users_array
                .iter()
                .filter_map(|user| user.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect()
        } else {
            Vec::new()
        };

        // Delete each user individually
        for user_id in user_ids {
            let delete_response = self.client
                .delete(&format!("{}/auth/v1/admin/users/{}", self.project_url, user_id))
                .header("Authorization", format!("Bearer {}", self.service_role_key))
                .header("apikey", &self.service_role_key)
                .send()
                .await
                .map_err(|e| AppError::ExternalService(format!("Failed to delete user {}: {}", user_id, e)))?;

            if !delete_response.status().is_success() {
                let error_text = delete_response.text().await.unwrap_or_default();
                eprintln!("Failed to delete user {}: {}", user_id, error_text);
                // Continue with other users even if one fails
            }
        }

        Ok(())
    }

    pub async fn debug_list_all_users(&self) -> Result<serde_json::Value, AppError> {
        let response = self.client
            .get(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to list users: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("Failed to list users: {}", error_text)));
        }

        let users: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse users list: {}", e)))?;

        eprintln!("All Supabase auth users: {}", serde_json::to_string_pretty(&users).unwrap_or_default());
        Ok(users)
    }

    pub async fn get_users_by_school_and_role(&self, school_id: &str, role: &str) -> Result<Vec<serde_json::Value>, AppError> {
        // Get all users and filter manually since Supabase admin API doesn't support direct filtering
        let response = self.client
            .get(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to list users: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("Failed to list users: {}", error_text)));
        }

        let users_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse users list: {}", e)))?;

        let empty_vec = vec![];
        let users = users_response
            .get("users")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let filtered_users: Vec<serde_json::Value> = users
            .iter()
            .filter(|user| {
                let metadata = user.get("user_metadata");
                if let Some(metadata) = metadata {
                    let user_school_id = metadata.get("school_id").and_then(|v| v.as_str());
                    let user_role = metadata.get("role").and_then(|v| v.as_str());

                    user_school_id == Some(school_id) && user_role == Some(role)
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        Ok(filtered_users)
    }

    pub async fn verify_jwt_and_get_user(&self, jwt_token: &str) -> Result<serde_json::Value, AppError> {
        let url = format!("{}/auth/v1/user", self.project_url);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .header("apikey", &self.anon_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to verify JWT: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Authorization(format!("Invalid JWT token: {}", error_text)));
        }

        let user_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user data: {}", e)))?;

        Ok(user_data)
    }


    pub async fn send_password_reset_email(&self, email: &str) -> Result<(), AppError> {
        let endpoint = format!("{}/auth/v1/recover", self.project_url);
        let body = json!({ "email": email });

        let response = self.client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send password reset email: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("Failed to send password reset email: {}", error_text)));
        }

        Ok(())
    }

    /// Call Supabase Admin generate_link to get a fresh one-time set-password URL for an existing user.
    /// Uses type "recovery" because the user already exists (created via admin API).
    /// "signup" fails with email_exists; "recovery" works for users with no password yet too.
    pub async fn generate_signup_link(&self, email: &str) -> Result<String, AppError> {
        let body = json!({
            "type": "recovery",
            "email": email,
            "options": {
                "redirectTo": format!("{}/set-password", self.frontend_url)
            }
        });

        let response = self.client
            .post(&format!("{}/auth/v1/admin/generate_link", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to call generate_link: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("generate_link failed: {}", error_text)));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse generate_link response: {}", e)))?;

        let action_link = data
            .get("action_link")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ExternalService("action_link not found in generate_link response".to_string()))?;

        Ok(action_link.to_string())
    }

    pub async fn send_parent_invite_email(
        &self,
        email: &str,
        invite_token: Uuid,
        first_name: &str,
        last_name: &str,
    ) -> Result<bool, AppError> {
        let confirmation_url = format!("{}/enrollments/activate/{}", self.api_base_url, invite_token);
        let html_body = super::email_templates::parent_invite_html(first_name, last_name, &confirmation_url);
        match self.email_service.dispatch(email, "Invitation to Create an Account for The Goddard School Admission", &html_body).await {
            Ok(_) => {
                tracing::info!("✅ Parent invite email sent to {} (token: {})", email, invite_token);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Parent invite email failed for {}: {}", email, e);
                Ok(false)
            }
        }
    }

    pub async fn send_admin_invite_email(
        &self,
        email: &str,
        invite_token: Uuid,
        first_name: &str,
        last_name: &str,
        school_name: &str,
    ) -> Result<bool, AppError> {
        let confirmation_url = format!("{}/enrollments/activate/{}", self.api_base_url, invite_token);
        let subject = format!("Welcome to {} - Administrator Access", school_name);
        let html_body = super::email_templates::admin_invite_html(
            first_name,
            last_name,
            school_name,
            &confirmation_url,
        );
        match self.email_service.dispatch(email, &subject, &html_body).await {
            Ok(_) => {
                tracing::info!("✅ Admin invite email sent to {} (token: {})", email, invite_token);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Admin invite email failed for {}: {}", email, e);
                Ok(false)
            }
        }
    }

    /// Create a Supabase user without sending any email (user creation only).
    pub async fn create_user_only_in_supabase(
        &self,
        email: &str,
        metadata: UserMetadata,
    ) -> Result<String, AppError> {
        let user_metadata_json = metadata.to_supabase_metadata();

        let create_user_body = json!({
            "email": email,
            "email_confirm": false,
            "user_metadata": user_metadata_json,
            "app_metadata": {
                "provider": "email",
                "providers": ["email"]
            }
        });

        let create_response = self.client
            .post(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&create_user_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to create user: {}", e)))?;

        let status_code = create_response.status();

        if !status_code.is_success() {
            let error_body = create_response.text().await.unwrap_or_default();
            if status_code == 422 && error_body.contains("already been registered") {
                return Err(AppError::Conflict("User with this email already exists".to_string()));
            }
            return Err(AppError::ExternalService(format!("Failed to create user: {}", error_body)));
        }

        let user_data: serde_json::Value = create_response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user creation response: {}", e)))?;

        let user_id = user_data
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| AppError::ExternalService("User ID not found in response".to_string()))?;

        tracing::info!("User created in Supabase: {}", user_id);
        Ok(user_id.to_string())
    }

    /// Create a Supabase user with a pre-set password (email immediately confirmed).
    /// Used by the bulk CSV import flow where passwords are auto-generated.
    pub async fn create_user_with_password_in_supabase(
        &self,
        email: &str,
        password: &str,
        metadata: UserMetadata,
    ) -> Result<String, AppError> {
        let user_metadata_json = metadata.to_supabase_metadata();

        let create_user_body = json!({
            "email": email,
            "password": password,
            "email_confirm": true,
            "user_metadata": user_metadata_json,
            "app_metadata": {
                "provider": "email",
                "providers": ["email"]
            }
        });

        let create_response = self.client
            .post(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&create_user_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to create user with password: {}", e)))?;

        let status_code = create_response.status();

        if !status_code.is_success() {
            let error_body = create_response.text().await.unwrap_or_default();
            if status_code == 422 && error_body.contains("already been registered") {
                return Err(AppError::Conflict("User with this email already exists".to_string()));
            }
            return Err(AppError::ExternalService(format!("Failed to create user with password: {}", error_body)));
        }

        let user_data: serde_json::Value = create_response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse user creation response: {}", e)))?;

        let user_id = user_data
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| AppError::ExternalService("User ID not found in response".to_string()))?;

        tracing::info!("User created with password in Supabase: {}", user_id);
        Ok(user_id.to_string())
    }

    pub async fn delete_user_by_id(&self, user_id: Uuid) -> Result<(), AppError> {
        let response = self.client
            .delete(&format!("{}/auth/v1/admin/users/{}", self.project_url, user_id))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to delete auth user during cleanup: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("Failed to delete auth user during cleanup: {}", body)));
        }
        Ok(())
    }

    pub async fn send_bulk_import_welcome_email(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        password: &str,
        school_name: &str,
    ) -> Result<bool, AppError> {
        let dashboard_url = self.frontend_url.clone();
        let html_body = super::email_templates::bulk_import_welcome_html(
            first_name,
            last_name,
            email,
            password,
            school_name,
            &dashboard_url,
        );
        let subject = format!("Welcome to {} — Your Login Details", school_name);
        match self.email_service.dispatch(email, &subject, &html_body).await {
            Ok(_) => {
                tracing::info!("✅ Bulk import welcome email sent to {}", email);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Bulk import welcome email failed for {}: {}", email, e);
                Ok(false)
            }
        }
    }
}
