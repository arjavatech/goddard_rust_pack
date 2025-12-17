use crate::error::AppError;
use reqwest::Client;
use serde_json::json;
use std::env;
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
}

#[derive(Clone)]
pub struct SupabaseClient {
    client: Client,
    project_url: String,
    service_role_key: String,
    anon_key: String,
}

impl SupabaseClient {
    pub fn new() -> Result<Self, AppError> {
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

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            project_url,
            service_role_key,
            anon_key,
        })
    }

    pub async fn resend_invitation(&self, email: &str) -> Result<(), AppError> {
        // For existing users, use the magic link endpoint to resend invitation
        let invite_response = self.client
            .post(&format!("{}/auth/v1/magiclink", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&json!({
                "email": email
            }))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send invitation: {}", e)))?;

        if !invite_response.status().is_success() {
            let status_code = invite_response.status();
            let error_text = invite_response.text().await.unwrap_or_default();

            // Handle rate limiting with user-friendly message
            if status_code == 429 || error_text.contains("over_email_send_rate_limit") {
                return Err(AppError::ExternalService(
                    "Email rate limit exceeded. Please wait 60 seconds before sending another invitation to the same email address.".to_string()
                ));
            }

            return Err(AppError::ExternalService(format!("Failed to send invitation: {}", error_text)));
        }

        Ok(())
    }

    pub async fn create_user_invitation_enhanced(&self, email: &str, metadata: UserMetadata) -> Result<String, AppError> {
        // Determine email template type based on role
        let template_type = match metadata.role.as_deref() {
            Some("SuperAdmin") | Some("Admin") => "invite",
            Some("Parent") => "signup",
            _ => "signup",  // Default to parent template
        };

        tracing::info!("[SupabaseClient] Sending email to {} with template type: {}", email, template_type);

        // Convert UserMetadata to serde_json::Value
        let user_metadata = serde_json::to_value(&metadata)
            .map_err(|e| AppError::Internal(format!("Failed to serialize user metadata: {}", e)))?;

        self.create_user_invitation_with_template(email, Some(user_metadata), template_type).await
    }

    pub async fn create_user_invitation_with_template(&self, email: &str, user_metadata: Option<serde_json::Value>, template_type: &str) -> Result<String, AppError> {
        // DEBUG: Log service role key prefix for verification
        let key_len = self.service_role_key.len();
        let prefix_len = std::cmp::min(30, key_len);
        tracing::info!("🔑 Creating user with service_role_key (prefix): {}...", &self.service_role_key[..prefix_len]);

        // Step 1: Create user with email_confirm: false (unconfirmed state)
        let mut create_request_body = json!({
            "email": email,
            "email_confirm": false
        });

        // Add user metadata if provided
        if let Some(metadata) = user_metadata {
            create_request_body["user_metadata"] = metadata;
        }

        let create_response = self.client
            .post(&format!("{}/auth/v1/admin/users", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&create_request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to create user: {}", e)))?;

        if !create_response.status().is_success() {
            let status_code = create_response.status();
            let error_text = create_response.text().await.unwrap_or_default();

            // Handle specific error cases
            if status_code == 422 && error_text.contains("already been registered") {
                return Err(AppError::Conflict("User with this email already exists".to_string()));
            }

            return Err(AppError::ExternalService(format!("Failed to create user: {}", error_text)));
        }

        let create_data: serde_json::Value = create_response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse create user response: {}", e)))?;

        // Response logged only in debug builds for performance
        #[cfg(debug_assertions)]
        eprintln!("Supabase create user response: {}", serde_json::to_string_pretty(&create_data).unwrap_or_default());

        let user_id = create_data
            .get("user")
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_str())
            .or_else(|| create_data.get("id").and_then(|id| id.as_str()))
            .ok_or_else(|| AppError::ExternalService(format!("User ID not found in response. Response: {}", serde_json::to_string(&create_data).unwrap_or_default())))?;

        // Step 2: Send signup confirmation email using the resend endpoint
        // Use role-based template type (invite for admins, signup for parents)
        let resend_request_body = json!({
            "email": email,
            "type": template_type,  // ROLE-BASED: "invite" or "signup"
            "options": {
                "emailRedirectTo": "https://your-domain.com/set_password.html"  // Same for all roles
            }
        });

        let resend_response = self.client
            .post(&format!("{}/auth/v1/resend", self.project_url))
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .header("Content-Type", "application/json")
            .json(&resend_request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send signup confirmation email: {}", e)))?;

        // Resend endpoint might return 200 even if rate limited, but we still want to proceed
        if !resend_response.status().is_success() {
            // Log the error but don't fail the entire operation since user was created
            #[cfg(debug_assertions)]
            eprintln!("Warning: Failed to send signup confirmation email, but user was created successfully. Status: {}", resend_response.status());
        }

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
}