use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::dao::DeviceTokenDao;

/// Drives Firebase Cloud Messaging HTTP v1 dispatch.
///
/// The DB insert for in-app notifications already happens inline (Lambda-safe).
/// This service handles the *out-of-process* push: signs a short-lived JWT,
/// trades it for a Google OAuth2 access token (cached ~50 min), and POSTs one
/// message per device token. UNREGISTERED responses prune the dead token row.
///
/// If the FCM_* env vars are missing on boot, `disabled()` returns a no-op
/// instance — the lambda still runs locally without Firebase configured.
pub struct FcmService {
    inner: FcmInner,
}

#[derive(Debug)]
pub enum PushDeliveryResult {
    Delivered,
    RetryableFailure(String),
    PermanentFailure(String),
}

enum FcmInner {
    Live(Box<LiveFcm>),
    Disabled,
}

struct LiveFcm {
    http: reqwest::Client,
    project_id: String,
    client_email: String,
    private_key_pem: String,
    device_token_dao: Arc<DeviceTokenDao>,
    cached: RwLock<Option<(String, Instant)>>,
}

#[derive(Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: u64,
}

impl FcmService {
    /// Build a live FCM dispatcher. `private_key_pem` may contain literal `\n`
    /// sequences (common when sourced from a single-line .env value) — they're
    /// normalized to real newlines here.
    pub fn live(
        project_id: String,
        client_email: String,
        private_key_pem: String,
        device_token_dao: Arc<DeviceTokenDao>,
    ) -> Self {
        let normalized = private_key_pem.replace("\\n", "\n");
        Self {
            inner: FcmInner::Live(Box::new(LiveFcm {
                http: reqwest::Client::new(),
                project_id,
                client_email,
                private_key_pem: normalized,
                device_token_dao,
                cached: RwLock::new(None),
            })),
        }
    }

    pub fn disabled() -> Self {
        Self {
            inner: FcmInner::Disabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self.inner, FcmInner::Live(_))
    }

    /// Send one durable outbox record. The caller owns retry policy; this method
    /// only classifies the FCM result and removes tokens that FCM has retired.
    pub async fn send_to_token(
        &self,
        token: &str,
        title: &str,
        body: &str,
        action_url: Option<&str>,
        notification_id: Uuid,
        notification_type: &str,
    ) -> PushDeliveryResult {
        let live = match &self.inner {
            FcmInner::Live(l) => l.as_ref(),
            FcmInner::Disabled => {
                return PushDeliveryResult::RetryableFailure("FCM is not configured".to_string())
            }
        };
        live.send_one(
            token,
            title,
            body,
            action_url,
            notification_id,
            notification_type,
        )
        .await
    }
}

impl LiveFcm {
    async fn send_one(
        &self,
        token: &str,
        title: &str,
        body: &str,
        action_url: Option<&str>,
        notification_id: Uuid,
        notification_type: &str,
    ) -> PushDeliveryResult {
        let access_token = match self.access_token().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[FcmService] could not obtain access token: {}", e);
                return PushDeliveryResult::RetryableFailure(e);
            }
        };
        send_one(
            &self.http,
            &access_token,
            &self.project_id,
            &self.device_token_dao,
            token,
            title,
            body,
            action_url,
            notification_id,
            notification_type,
        )
        .await
    }

    async fn access_token(&self) -> Result<String, String> {
        // Cache hit?
        if let Some((tok, exp)) = self.cached.read().await.as_ref() {
            if Instant::now() < *exp - Duration::from_secs(60) {
                return Ok(tok.clone());
            }
        }

        // Sign + exchange.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let claims = JwtClaims {
            iss: &self.client_email,
            scope: "https://www.googleapis.com/auth/firebase.messaging",
            aud: "https://oauth2.googleapis.com/token",
            iat: now,
            exp: now + 3600,
        };

        let key = EncodingKey::from_rsa_pem(self.private_key_pem.as_bytes())
            .map_err(|e| format!("invalid FCM private key: {}", e))?;
        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)
            .map_err(|e| format!("JWT encode failed: {}", e))?;

        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| format!("token exchange request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("token exchange {}: {}", status, text));
        }

        let parsed: GoogleTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("token exchange parse failed: {}", e))?;

        let expires_at = Instant::now() + Duration::from_secs(parsed.expires_in.saturating_sub(60));
        *self.cached.write().await = Some((parsed.access_token.clone(), expires_at));
        Ok(parsed.access_token)
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_one(
    http: &reqwest::Client,
    access_token: &str,
    project_id: &str,
    dao: &DeviceTokenDao,
    token: &str,
    title: &str,
    body: &str,
    action_url: Option<&str>,
    notification_id: Uuid,
    notification_type: &str,
) -> PushDeliveryResult {
    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        project_id
    );

    // Data-only payload. We intentionally do NOT send a top-level `notification`
    // field: when both are present, Chrome auto-displays the notification in
    // background AND our service worker's onBackgroundMessage also calls
    // showNotification() — the user sees the same alert twice. With data-only,
    // the SW is the sole renderer and we get exactly one popup.
    let mut data_with_text = serde_json::Map::new();
    data_with_text.insert(
        "notification_id".to_string(),
        json!(notification_id.to_string()),
    );
    data_with_text.insert("type".to_string(), json!(notification_type));
    data_with_text.insert("title".to_string(), json!(title));
    data_with_text.insert("body".to_string(), json!(body));
    if let Some(action_url) = action_url {
        data_with_text.insert("action_url".to_string(), json!(action_url));
    }

    let mut webpush_fcm_options = serde_json::Map::new();
    if let Some(link) = action_url {
        webpush_fcm_options.insert(
            "link".to_string(),
            serde_json::Value::String(link.to_string()),
        );
    }

    let payload = json!({
        "message": {
            "token": token,
            "data": data_with_text,
            "webpush": {
                "fcm_options": webpush_fcm_options
            }
        }
    });

    let resp = match http
        .post(url)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "[FcmService] HTTP send failed for token {}…: {}",
                short(token),
                e
            );
            return PushDeliveryResult::RetryableFailure(e.to_string());
        }
    };

    let status = resp.status();
    if status.is_success() {
        return PushDeliveryResult::Delivered;
    }

    let body_text = resp.text().await.unwrap_or_default();
    // FCM HTTP v1 returns 404 NOT_FOUND for unregistered tokens, and 400 with
    // error.status="INVALID_ARGUMENT" when the token is malformed. Both are
    // terminal — prune the row so we stop hammering.
    let unregistered = status.as_u16() == 404
        || body_text.contains("UNREGISTERED")
        || body_text.contains("registration-token-not-registered")
        || body_text.contains("INVALID_ARGUMENT");

    if unregistered {
        eprintln!(
            "[FcmService] token {}… is dead ({}); deleting",
            short(token),
            status
        );
        if let Err(e) = dao.delete_token(token).await {
            eprintln!("[FcmService] failed to delete dead token: {:?}", e);
        }
        return PushDeliveryResult::PermanentFailure(format!("FCM rejected token: {}", status));
    } else {
        eprintln!(
            "[FcmService] send failed ({}): {}",
            status,
            body_text.chars().take(300).collect::<String>()
        );
    }
    PushDeliveryResult::RetryableFailure(format!(
        "FCM send failed ({}): {}",
        status,
        body_text.chars().take(300).collect::<String>()
    ))
}

fn short(token: &str) -> String {
    token.chars().take(12).collect()
}
