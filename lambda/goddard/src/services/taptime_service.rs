use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Clone)]
pub struct TapTimeService {
    client: Client,
    base_url: String,
    client_id: String,
    client_secret: String,
    token: Arc<Mutex<Option<CachedToken>>>,
}
struct CachedToken {
    value: String,
    expires_at: Instant,
}
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl TapTimeService {
    pub fn from_env() -> Result<Self, AppError> {
        let base_url = env::var("TAPTIME_API_BASE_URL")
            .map_err(|_| AppError::Internal("TAPTIME_API_BASE_URL is required".into()))?
            .trim_end_matches('/')
            .to_string();
        let client_id = env::var("TAPTIME_PROVISIONING_CLIENT_ID")
            .map_err(|_| AppError::Internal("TAPTIME_PROVISIONING_CLIENT_ID is required".into()))?;
        let client_secret = env::var("TAPTIME_PROVISIONING_CLIENT_SECRET").map_err(|_| {
            AppError::Internal("TAPTIME_PROVISIONING_CLIENT_SECRET is required".into())
        })?;
        let timeout = env::var("TAPTIME_REQUEST_TIMEOUT_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout))
                .build()
                .map_err(|e| AppError::Internal(e.to_string()))?,
            base_url,
            client_id,
            client_secret,
            token: Arc::new(Mutex::new(None)),
        })
    }

    async fn access_token(&self, force_refresh: bool) -> Result<String, AppError> {
        let mut cache = self.token.lock().await;
        if !force_refresh {
            if let Some(token) = cache.as_ref() {
                if token.expires_at > Instant::now() + Duration::from_secs(30) {
                    return Ok(token.value.clone());
                }
            }
        }
        let response = self
            .client
            .post(format!("{}/v1/integration-auth/token", self.base_url))
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("TapTime token request failed: {e}")))?;
        if !response.status().is_success() {
            return Err(AppError::ExternalService(format!(
                "TapTime token request returned {}",
                response.status()
            )));
        }
        let payload: TokenResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Invalid TapTime token response: {e}"))
        })?;
        let value = payload.access_token.clone();
        *cache = Some(CachedToken {
            value: payload.access_token,
            expires_at: Instant::now() + Duration::from_secs(payload.expires_in),
        });
        Ok(value)
    }

    pub async fn deliver(
        &self,
        school_id: Uuid,
        employee_id: Uuid,
        action: &str,
        payload: &Value,
        idempotency_key: Uuid,
    ) -> Result<Value, AppError> {
        let path = match action {
            "upsert" => format!("/v1/integrations/provisioning/employees/{employee_id}"),
            "pin" => format!("/v1/integrations/provisioning/employees/{employee_id}/pin"),
            "status" => format!("/v1/integrations/provisioning/employees/{employee_id}/status"),
            _ => return Err(AppError::Validation("Unknown TapTime sync action".into())),
        };
        for refresh in [false, true] {
            let token = self.access_token(refresh).await?;
            let request = self
                .client
                .request(
                    if action == "upsert" {
                        reqwest::Method::PUT
                    } else {
                        reqwest::Method::PATCH
                    },
                    format!("{}{}", self.base_url, path),
                )
                .bearer_auth(token)
                .header("X-Integration-Tenant", school_id.to_string())
                .header("Idempotency-Key", idempotency_key.to_string())
                .json(payload);
            let response = request.send().await.map_err(|e| {
                AppError::ExternalService(format!("TapTime sync request failed: {e}"))
            })?;
            if response.status().is_success() {
                return response.json().await.map_err(|e| {
                    AppError::ExternalService(format!("Invalid TapTime sync response: {e}"))
                });
            }
            if response.status() == StatusCode::UNAUTHORIZED && !refresh {
                continue;
            }
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "TapTime sync returned {status}: {body}"
            )));
        }
        unreachable!()
    }

    /// Server-to-server lookup for the explicit mapping screen. The browser never
    /// receives the provisioning credential or calls TapTime directly.
    pub async fn available_employees(&self, school_id: Uuid) -> Result<Vec<Value>, AppError> {
        let payload = self
            .request_json(
                reqwest::Method::GET,
                "/v1/integrations/provisioning/employees",
                school_id,
                None,
                None,
            )
            .await?;
        Ok(payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    pub async fn employment_types(&self, school_id: Uuid) -> Result<Vec<String>, AppError> {
        let payload = self.request_json(
            reqwest::Method::GET,
            "/v1/integrations/provisioning/attendance/employment-types",
            school_id,
            None,
            None,
        ).await?;
        Ok(payload.get("items").and_then(Value::as_array).into_iter().flatten()
            .filter_map(Value::as_str).map(str::to_string).collect())
    }

    /// Creates only TapTime's integration link. It does not alter the TapTime
    /// employee record, so mapping an existing user is safe and deliberate.
    pub async fn link_existing_employee(
        &self,
        school_id: Uuid,
        goddard_user_id: Uuid,
        taptime_emp_id: Uuid,
    ) -> Result<Value, AppError> {
        self.request_json(
            reqwest::Method::POST,
            &format!("/v1/integrations/provisioning/employees/{goddard_user_id}/link"),
            school_id,
            Some(serde_json::json!({
                "emp_id": taptime_emp_id,
                "external_auth_subject": goddard_user_id.to_string(),
            })),
            Some(Uuid::new_v4()),
        )
        .await
    }

    /// Redeem the one-time customer pairing code.  It is submitted from the
    /// Goddard backend with its provisioning token, never from a browser.
    pub async fn redeem_tenant_pairing_code(&self, school_id: Uuid, code: &str) -> Result<Value, AppError> {
        for refresh in [false, true] {
            let token = self.access_token(refresh).await?;
            let response = self.client
                .post(format!("{}/v1/integration-auth/tenant-pairing/redeem", self.base_url))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "code": code,
                    "external_tenant_id": school_id.to_string(),
                }))
                .send().await
                .map_err(|e| AppError::ExternalService(format!("TapTime pairing request failed: {e}")))?;
            if response.status().is_success() {
                return response.json().await.map_err(|e| AppError::ExternalService(format!("Invalid TapTime pairing response: {e}")));
            }
            if response.status() == StatusCode::UNAUTHORIZED && !refresh { continue; }
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("TapTime pairing returned {status}: {body}")));
        }
        unreachable!()
    }

    async fn request_json(
        &self,
        method: reqwest::Method,
        path: &str,
        school_id: Uuid,
        payload: Option<Value>,
        idempotency_key: Option<Uuid>,
    ) -> Result<Value, AppError> {
        for refresh in [false, true] {
            let token = self.access_token(refresh).await?;
            let mut request = self
                .client
                .request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(token)
                .header("X-Integration-Tenant", school_id.to_string());
            if let Some(key) = idempotency_key {
                request = request.header("Idempotency-Key", key.to_string());
            }
            if let Some(body) = payload.as_ref() {
                request = request.json(body);
            }
            let response = request
                .send()
                .await
                .map_err(|e| AppError::ExternalService(format!("TapTime request failed: {e}")))?;
            if response.status().is_success() {
                return response.json().await.map_err(|e| {
                    AppError::ExternalService(format!("Invalid TapTime response: {e}"))
                });
            }
            if response.status() == StatusCode::UNAUTHORIZED && !refresh {
                continue;
            }
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "TapTime returned {status}: {body}"
            )));
        }
        unreachable!()
    }
}
