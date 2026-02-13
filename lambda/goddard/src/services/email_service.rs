use crate::{
    error::{AppError, ApiResult},
    models::email::{ParentFormReminder, BulkEmailResponse},
};
use chrono::NaiveDate;
use serde_json::json;

pub struct EmailService {
    client: reqwest::Client,
    api_key: String,
    from_email: String,
}

impl EmailService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: "re_eTyTdH1N_GD3g5HKBTT9yXp5dcAVPsZxK".to_string(),
            from_email: "Goddard Schools <no-reply@arjavatech.com>".to_string(),
        }
    }

    /// Convert DD-MM-YYYY to "December 25, 2025" format
    fn convert_date_format(dd_mm_yyyy: &str) -> Result<String, AppError> {
        // Parse DD-MM-YYYY
        let parts: Vec<&str> = dd_mm_yyyy.split('-').collect();
        if parts.len() != 3 {
            return Err(AppError::Validation("Invalid date format. Expected DD-MM-YYYY".to_string()));
        }

        let day: u32 = parts[0].parse()
            .map_err(|_| AppError::Validation("Invalid day in date".to_string()))?;
        let month: u32 = parts[1].parse()
            .map_err(|_| AppError::Validation("Invalid month in date".to_string()))?;
        let year: i32 = parts[2].parse()
            .map_err(|_| AppError::Validation("Invalid year in date".to_string()))?;

        // Create NaiveDate
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| AppError::Validation("Invalid date values".to_string()))?;

        // Format as "December 25, 2025"
        Ok(date.format("%B %d, %Y").to_string())
    }

    /// Generate HTML email body
    fn generate_html_body(
        parent_name: &str,
        student_name: &str,
        class_name: &str,
        form_name: &str,
        formatted_due_date: &str,
    ) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Form Reminder</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f8f9fa; border-left: 4px solid #0066cc; padding: 20px; margin-bottom: 20px;">
        <h2 style="color: #0066cc; margin-top: 0;">Form Reminder</h2>
    </div>

    <p>Hi <strong>{}</strong>,</p>

    <p>Your child <strong>{}</strong> is enrolled in <strong>{}</strong> at Goddard School.</p>

    <p>We noticed that the <strong>{}</strong> has not been completed yet. Please take a moment to fill out this important form by <strong>{}</strong>.</p>

    <p>If you have any questions, please don't hesitate to contact us.</p>

    <div style="margin-top: 30px; padding-top: 20px; border-top: 1px solid #ddd;">
        <p style="margin-bottom: 5px;"><strong>Best regards,</strong></p>
        <p style="margin-top: 5px; color: #0066cc;"><strong>Goddard School Team</strong></p>
    </div>
</body>
</html>"#,
            parent_name, student_name, class_name, form_name, formatted_due_date
        )
    }

    /// Send a single form reminder email
    async fn send_form_reminder_email(&self, reminder: &ParentFormReminder) -> Result<(), AppError> {
        println!("[EmailService] Sending email to: {}", reminder.parent_email);

        // Convert date format, fallback for empty due_date
        let formatted_due_date = if reminder.due_date.trim().is_empty() {
            "at your earliest convenience".to_string()
        } else {
            Self::convert_date_format(&reminder.due_date)?
        };
        println!("[EmailService] Converted date: {} → {}", reminder.due_date, formatted_due_date);

        // Generate email body
        let html_body = Self::generate_html_body(
            &reminder.parent_name,
            &reminder.student_name,
            &reminder.class_name,
            &reminder.form_name,
            &formatted_due_date,
        );

        // Generate subject — omit "Due" when no due_date provided
        let subject = if reminder.due_date.trim().is_empty() {
            format!("Reminder: {} for {}", reminder.form_name, reminder.student_name)
        } else {
            format!("Reminder: {} for {} - Due {}", reminder.form_name, reminder.student_name, formatted_due_date)
        };

        // Split comma-separated emails and trim whitespace
        let emails: Vec<String> = reminder.parent_email
            .split(',')
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty())
            .collect();

        // Prepare Resend API request
        let request_body = json!({
            "from": self.from_email,
            "to": emails,
            "subject": subject,
            "html": html_body
        });

        // Call Resend API
        let response = self.client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send email: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            println!("[EmailService] Email send failed: {} - {}", status, error_text);
            return Err(AppError::ExternalService(format!(
                "Resend API error: {} - {}",
                status, error_text
            )));
        }

        let response_data: serde_json::Value = response.json().await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse response: {}", e)))?;

        println!("[EmailService] Email sent successfully: {:?}", response_data);
        Ok(())
    }

    /// Send bulk form reminder emails
    pub async fn send_bulk_form_reminders(
        &self,
        reminders: Vec<ParentFormReminder>,
    ) -> ApiResult<BulkEmailResponse> {
        println!("[EmailService] Starting bulk email send: {} emails", reminders.len());

        let mut total_sent = 0;
        let mut total_failed = 0;
        let mut failed_emails = Vec::new();

        for reminder in reminders {
            match self.send_form_reminder_email(&reminder).await {
                Ok(_) => {
                    total_sent += 1;
                }
                Err(e) => {
                    println!("[EmailService] Failed to send to {}: {:?}", reminder.parent_email, e);
                    total_failed += 1;
                    failed_emails.push(reminder.parent_email.clone());
                }
            }
        }

        let message = if total_failed == 0 {
            format!("Successfully sent {} emails", total_sent)
        } else {
            format!(
                "Sent {} emails successfully, {} failed",
                total_sent, total_failed
            )
        };

        println!("[EmailService] Bulk send complete: {}", message);

        Ok(BulkEmailResponse {
            total_sent,
            total_failed,
            failed_emails,
            message,
        })
    }
}
