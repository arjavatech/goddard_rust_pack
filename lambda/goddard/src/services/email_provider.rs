use crate::error::AppError;
use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, to: Vec<String>, subject: String, html: String) -> Result<(), AppError>;
    async fn send_batch(
        &self,
        subject: String,
        html: String,
        recipients: Vec<BatchRecipient>,
    ) -> Result<(), AppError>;
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchRecipient {
    pub address: String,
    pub name: String,
    pub merge_info: serde_json::Value,
}

pub struct ZeptoMailProvider {
    client: reqwest::Client,
    token: String,
    from_name: String,
    from_address: String,
}

impl ZeptoMailProvider {
    pub fn new() -> Result<Self, AppError> {
        let raw_token = std::env::var("ZEPTOMAIL_SEND_MAIL_TOKEN").map_err(|_| {
            AppError::Internal(
                "ZEPTOMAIL_SEND_MAIL_TOKEN is required when EMAIL_PROVIDER=zeptomail".to_string(),
            )
        })?;
        let from = std::env::var("EMAIL_FROM")
            .unwrap_or_else(|_| "Goddard Schools <no-reply@arjavatech.com>".to_string());
        let mailbox: lettre::message::Mailbox = from
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid EMAIL_FROM: {}", e)))?;
        let token = raw_token
            .trim()
            .strip_prefix("Zoho-enczapikey ")
            .unwrap_or(raw_token.trim())
            .to_string();
        Ok(Self {
            client: reqwest::Client::new(),
            token,
            from_name: mailbox
                .name
                .unwrap_or_else(|| "Goddard Schools".to_string()),
            from_address: mailbox.email.to_string(),
        })
    }
}

#[derive(Deserialize)]
struct ZeptoMailResponse {
    request_id: Option<String>,
}

#[async_trait]
impl EmailProvider for ZeptoMailProvider {
    async fn send(&self, to: Vec<String>, subject: String, html: String) -> Result<(), AppError> {
        if to.is_empty() {
            return Err(AppError::Validation(
                "No recipient email addresses provided".to_string(),
            ));
        }
        let recipients: Vec<_> = to
            .into_iter()
            .map(|address| json!({"email_address": {"address": address}}))
            .collect();
        self.send_request("https://api.zeptomail.com/v1.1/email", json!({"from":{"address":self.from_address,"name":self.from_name},"to":recipients,"subject":subject,"htmlbody":html,"track_clicks":false,"track_opens":false})).await
    }

    async fn send_batch(
        &self,
        subject: String,
        html: String,
        recipients: Vec<BatchRecipient>,
    ) -> Result<(), AppError> {
        if recipients.is_empty() {
            return Err(AppError::Validation(
                "No recipient email addresses provided".to_string(),
            ));
        }
        // ZeptoMail's batch endpoint accepts at most 500 recipients. Splitting
        // here preserves the calling API while allowing a request with more
        // than 500 comma-separated parent addresses.
        for chunk in recipients.chunks(500) {
            let to: Vec<_> = chunk
                .iter()
                .map(|recipient| {
                    json!({"email_address":{"address":recipient.address,"name":recipient.name},"merge_info":recipient.merge_info})
                })
                .collect();
            self.send_request("https://api.zeptomail.com/v1.1/email/batch", json!({"from":{"address":self.from_address,"name":self.from_name},"to":to,"subject":subject,"htmlbody":html,"track_clicks":false,"track_opens":false})).await?;
        }
        Ok(())
    }
}

impl ZeptoMailProvider {
    async fn send_request(&self, url: &str, payload: serde_json::Value) -> Result<(), AppError> {
        let response = self
            .client
            .post(url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Zoho-enczapikey {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("ZeptoMail request failed: {}", e)))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::ExternalService(format!(
                "ZeptoMail rejected email ({}): {}",
                status, body
            )));
        }
        let request_id = serde_json::from_str::<ZeptoMailResponse>(&body)
            .ok()
            .and_then(|value| value.request_id);
        println!("[ZeptoMail] Email accepted: request_id={:?}", request_id);
        Ok(())
    }
}

pub struct SmtpProvider {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl SmtpProvider {
    pub fn new() -> Self {
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.zoho.com".to_string());
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);
        let user = std::env::var("SMTP_USER").unwrap_or_default();
        let pass = std::env::var("SMTP_PASS").unwrap_or_default();
        let from = std::env::var("EMAIL_FROM")
            .unwrap_or_else(|_| "Goddard Schools <no-reply@arjavatech.com>".to_string());

        let creds = Credentials::new(user, pass);
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .expect("Failed to create SMTP transport — check SMTP_HOST")
            .port(port)
            .credentials(creds)
            .timeout(Some(Duration::from_secs(30)))
            .build();

        Self { mailer, from }
    }
}

#[async_trait]
impl EmailProvider for SmtpProvider {
    async fn send(&self, to: Vec<String>, subject: String, html: String) -> Result<(), AppError> {
        if to.is_empty() {
            return Err(AppError::Validation(
                "No recipient email addresses provided".to_string(),
            ));
        }

        let from_mailbox: lettre::message::Mailbox = self.from.parse().map_err(|e| {
            AppError::Internal(format!("Invalid FROM address '{}': {}", self.from, e))
        })?;

        let mut builder = Message::builder()
            .from(from_mailbox)
            .subject(&subject)
            .header(ContentType::TEXT_HTML);

        for addr in &to {
            let mailbox: lettre::message::Mailbox = addr.parse().map_err(|e| {
                AppError::Validation(format!("Invalid recipient '{}': {}", addr, e))
            })?;
            builder = builder.to(mailbox);
        }

        let email = builder
            .body(html)
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        self.mailer
            .send(email)
            .await
            .map_err(|e| AppError::ExternalService(format!("SMTP send failed: {}", e)))?;

        Ok(())
    }

    async fn send_batch(
        &self,
        subject: String,
        html: String,
        recipients: Vec<BatchRecipient>,
    ) -> Result<(), AppError> {
        for recipient in recipients {
            let mut personalized_subject = subject.clone();
            let mut personalized_html = html.clone();
            let values = recipient
                .merge_info
                .as_object()
                .ok_or_else(|| AppError::Internal("Invalid batch merge values".to_string()))?;
            for (key, value) in values {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = value.as_str().unwrap_or_default();
                personalized_subject = personalized_subject.replace(&placeholder, replacement);
                personalized_html = personalized_html.replace(&placeholder, replacement);
            }
            self.send(
                vec![recipient.address],
                personalized_subject,
                personalized_html,
            )
            .await?;
        }
        Ok(())
    }
}
