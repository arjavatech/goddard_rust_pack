use crate::error::AppError;
use async_trait::async_trait;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::time::Duration;

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, to: Vec<String>, subject: String, html: String) -> Result<(), AppError>;
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

        let from_mailbox: lettre::message::Mailbox = self
            .from
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid FROM address '{}': {}", self.from, e)))?;

        let mut builder = Message::builder()
            .from(from_mailbox)
            .subject(&subject)
            .header(ContentType::TEXT_HTML);

        for addr in &to {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .map_err(|e| AppError::Validation(format!("Invalid recipient '{}': {}", addr, e)))?;
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
}
