use crate::error::AppError;
use reqwest::Client;
use serde_json::json;
use std::env;

pub struct SupabaseClient {
    client: Client,
    project_url: String,
    service_role_key: String,
}

impl SupabaseClient {
    pub fn new() -> Result<Self, AppError> {
        let project_url = env::var("SUPABASE_URL")
            .map_err(|_| AppError::Internal("SUPABASE_URL must be set".to_string()))?;
        let service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| AppError::Internal("SUPABASE_SERVICE_ROLE_KEY must be set".to_string()))?;

        // Check if the service role key is still the placeholder
        if service_role_key == "your_actual_service_role_key_here" {
            return Err(AppError::Internal(
                "SUPABASE_SERVICE_ROLE_KEY is still set to placeholder. Please replace with actual service role key from Supabase dashboard.".to_string()
            ));
        }

        Ok(Self {
            client: Client::new(),
            project_url,
            service_role_key,
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

    pub async fn create_user_invitation(&self, email: &str, user_metadata: Option<serde_json::Value>) -> Result<String, AppError> {
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

        let user_id = create_data["user"]["id"]
            .as_str()
            .or_else(|| create_data["id"].as_str())
            .ok_or_else(|| AppError::ExternalService(format!("User ID not found in response. Response: {}", serde_json::to_string(&create_data).unwrap_or_default())))?;

        // Step 2: Send signup confirmation email using the resend endpoint
        let resend_request_body = json!({
            "email": email,
            "type": "signup"
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
            eprintln!("Warning: Failed to send signup confirmation email, but user was created successfully. Status: {}", resend_response.status());
        }

        Ok(user_id.to_string())
    }
}