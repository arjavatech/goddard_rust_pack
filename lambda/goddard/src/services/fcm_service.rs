use std::collections::HashMap;
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

    /// Send to every token registered for `user_id`. Errors logged, never propagated.
    pub async fn send_to_user(
        &self,
        user_id: Uuid,
        title: &str,
        body: &str,
        action_url: Option<&str>,
        related_entity_id: Option<Uuid>,
        notification_type: &str,
    ) {
        let live = match &self.inner {
            FcmInner::Live(l) => l.as_ref(),
            FcmInner::Disabled => return,
        };

        let tokens = match live.device_token_dao.tokens_for_user(user_id).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[FcmService] tokens_for_user({}) failed: {:?}", user_id, e);
                return;
            }
        };

        if tokens.is_empty() {
            return;
        }

        live.fan_out(tokens, title, body, action_url, related_entity_id, notification_type)
            .await;
    }

    /// Send to every token registered for any of `user_ids`. Batched DB read +
    /// concurrent HTTP fan-out.
    pub async fn send_to_users(
        &self,
        user_ids: &[Uuid],
        title: &str,
        body: &str,
        action_url: Option<&str>,
        related_entity_id: Option<Uuid>,
        notification_type: &str,
    ) {
        let live = match &self.inner {
            FcmInner::Live(l) => l.as_ref(),
            FcmInner::Disabled => return,
        };

        if user_ids.is_empty() {
            return;
        }

        let pairs = match live.device_token_dao.tokens_for_users(user_ids).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[FcmService] tokens_for_users failed: {:?}", e);
                return;
            }
        };

        if pairs.is_empty() {
            return;
        }

        let tokens: Vec<String> = pairs.into_iter().map(|(_, t)| t).collect();
        live.fan_out(tokens, title, body, action_url, related_entity_id, notification_type)
            .await;
    }
}

impl LiveFcm {
    async fn fan_out(
        &self,
        tokens: Vec<String>,
        title: &str,
        body: &str,
        action_url: Option<&str>,
        related_entity_id: Option<Uuid>,
        notification_type: &str,
    ) {
        let access_token = match self.access_token().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[FcmService] could not obtain access token: {}", e);
                return;
            }
        };

        let mut data = HashMap::new();
        if let Some(id) = related_entity_id {
            data.insert("notification_id".to_string(), id.to_string());
        }
        data.insert("type".to_string(), notification_type.to_string());
        if let Some(url) = action_url {
            data.insert("action_url".to_string(), url.to_string());
        }

        let mut ok = 0usize;
        let total = tokens.len();
        let mut handles = Vec::with_capacity(total);

        for token in tokens {
            let http = self.http.clone();
            let access = access_token.clone();
            let project_id = self.project_id.clone();
            let dao = self.device_token_dao.clone();
            let title = title.to_string();
            let body = body.to_string();
            let action_url_owned = action_url.map(|s| s.to_string());
            let data = data.clone();

            handles.push(tokio::spawn(async move {
                send_one(
                    &http,
                    &access,
                    &project_id,
                    &dao,
                    &token,
                    &title,
                    &body,
                    action_url_owned.as_deref(),
                    &data,
                )
                .await
            }));
        }

        for h in handles {
            if let Ok(true) = h.await {
                ok += 1;
            }
        }

        println!(
            "[FcmService] sent {}/{} (type={})",
            ok, total, notification_type
        );
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
    data: &HashMap<String, String>,
) -> bool {
    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        project_id
    );

    // Data-only payload. We intentionally do NOT send a top-level `notification`
    // field: when both are present, Chrome auto-displays the notification in
    // background AND our service worker's onBackgroundMessage also calls
    // showNotification() — the user sees the same alert twice. With data-only,
    // the SW is the sole renderer and we get exactly one popup.
    let mut data_with_text = data.clone();
    data_with_text.insert("title".to_string(), title.to_string());
    data_with_text.insert("body".to_string(), body.to_string());

    let mut webpush_fcm_options = serde_json::Map::new();
    if let Some(link) = action_url {
        webpush_fcm_options.insert("link".to_string(), serde_json::Value::String(link.to_string()));
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
            eprintln!("[FcmService] HTTP send failed for token {}…: {}", short(token), e);
            return false;
        }
    };

    let status = resp.status();
    if status.is_success() {
        return true;
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
    } else {
        eprintln!(
            "[FcmService] send failed ({}): {}",
            status,
            body_text.chars().take(300).collect::<String>()
        );
    }
    false
}

fn short(token: &str) -> String {
    token.chars().take(12).collect()
}
