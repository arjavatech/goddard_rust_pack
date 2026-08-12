use crate::{
    error::{AppError, ApiResult},
    models::email::{
        BulkEmailResponse, ChildAddedNotification, ChildArchivedNotification,
        FormApprovedNotification, FormAssignedNotification, FormRejectedNotification,
        ParentDeactivatedNotification, ParentFormReminder,
    },
    services::{email_provider::{EmailProvider, SmtpProvider}, email_templates},
};
use chrono::NaiveDate;
use std::sync::Arc;

pub struct EmailService {
    provider: Arc<dyn EmailProvider>,
}

impl EmailService {
    pub fn new() -> Self {
        Self {
            provider: Arc::new(SmtpProvider::new()),
        }
    }

    /// Dispatch a single email. Accepts comma-separated recipients in `to`.
    pub async fn dispatch(&self, to: &str, subject: &str, html: &str) -> Result<(), AppError> {
        let emails: Vec<String> = to
            .split(',')
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty())
            .collect();

        if emails.is_empty() {
            return Err(AppError::Validation(
                "No recipient email addresses provided".to_string(),
            ));
        }

        println!(
            "[EmailService] Sending: subject={:?}, to={:?}",
            subject, emails
        );

        self.provider
            .send(emails, subject.to_string(), html.to_string())
            .await
    }

    /// Convert DD-MM-YYYY to "December 25, 2025" format
    fn convert_date_format(dd_mm_yyyy: &str) -> Result<String, AppError> {
        let parts: Vec<&str> = dd_mm_yyyy.split('-').collect();
        if parts.len() != 3 {
            return Err(AppError::Validation(
                "Invalid date format. Expected DD-MM-YYYY".to_string(),
            ));
        }

        let day: u32 = parts[0]
            .parse()
            .map_err(|_| AppError::Validation("Invalid day in date".to_string()))?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| AppError::Validation("Invalid month in date".to_string()))?;
        let year: i32 = parts[2]
            .parse()
            .map_err(|_| AppError::Validation("Invalid year in date".to_string()))?;

        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| AppError::Validation("Invalid date values".to_string()))?;

        Ok(date.format("%B %d, %Y").to_string())
    }

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

    async fn send_form_reminder_email(&self, reminder: &ParentFormReminder) -> Result<(), AppError> {
        println!("[EmailService] Sending email to: {}", reminder.parent_email);

        let formatted_due_date = if reminder.due_date.trim().is_empty() {
            "at your earliest convenience".to_string()
        } else {
            Self::convert_date_format(&reminder.due_date)?
        };
        println!(
            "[EmailService] Converted date: {} → {}",
            reminder.due_date, formatted_due_date
        );

        let html_body = Self::generate_html_body(
            &reminder.parent_name,
            &reminder.student_name,
            &reminder.class_name,
            &reminder.form_name,
            &formatted_due_date,
        );

        let subject = if reminder.due_date.trim().is_empty() {
            format!("Reminder: {} for {}", reminder.form_name, reminder.student_name)
        } else {
            format!(
                "Reminder: {} for {} - Due {}",
                reminder.form_name, reminder.student_name, formatted_due_date
            )
        };

        self.dispatch(&reminder.parent_email, &subject, &html_body).await
    }

    pub async fn send_bulk_form_reminders(
        &self,
        reminders: Vec<ParentFormReminder>,
    ) -> ApiResult<BulkEmailResponse> {
        println!(
            "[EmailService] Starting bulk email send: {} emails",
            reminders.len()
        );

        let mut total_sent = 0;
        let mut total_failed = 0;
        let mut failed_emails = Vec::new();

        for reminder in reminders {
            match self.send_form_reminder_email(&reminder).await {
                Ok(_) => {
                    total_sent += 1;
                }
                Err(e) => {
                    println!(
                        "[EmailService] Failed to send to {}: {:?}",
                        reminder.parent_email, e
                    );
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

    // =====================================================
    // Parent lifecycle notifications
    // =====================================================

    pub async fn send_form_approved_email(
        &self,
        payload: FormApprovedNotification,
    ) -> Result<(), AppError> {
        let subject = format!(
            "Good news! {} has been approved for {}",
            payload.form_name, payload.child_name
        );
        let html = email_templates::form_approved_html(&payload);
        self.dispatch(&payload.parent_email, &subject, &html).await
    }

    pub async fn send_form_rejected_email(
        &self,
        payload: FormRejectedNotification,
    ) -> Result<(), AppError> {
        let subject = format!(
            "Action needed: {} for {} requires updates",
            payload.form_name, payload.child_name
        );
        let html = email_templates::form_rejected_html(&payload);
        self.dispatch(&payload.parent_email, &subject, &html).await
    }

    pub async fn send_child_added_email(
        &self,
        payload: ChildAddedNotification,
    ) -> Result<(), AppError> {
        let subject = format!(
            "{} has been added to your Goddard School account",
            payload.child_name
        );
        let html = email_templates::child_added_html(&payload);
        self.dispatch(&payload.parent_email, &subject, &html).await
    }

    pub async fn send_parent_deactivated_email(
        &self,
        payload: ParentDeactivatedNotification,
    ) -> Result<(), AppError> {
        let subject = "Your Goddard School parent account has been deactivated".to_string();
        let html = email_templates::parent_deactivated_html(&payload);
        self.dispatch(&payload.parent_email, &subject, &html).await
    }

    pub async fn send_child_archived_email(
        &self,
        payload: ChildArchivedNotification,
    ) -> Result<(), AppError> {
        let subject = format!(
            "{}'s record has been archived at {}",
            payload.child_name, payload.school_name
        );
        let html = email_templates::child_archived_html(&payload);
        self.dispatch(&payload.parent_email, &subject, &html).await
    }

    pub async fn send_form_assigned_email(
        &self,
        payload: FormAssignedNotification,
    ) -> Result<(), AppError> {
        let subject = format!(
            "New form for {}: please complete {}",
            payload.child_name, payload.form_name
        );
        let html = email_templates::form_assigned_html(&payload);
        self.dispatch(&payload.parent_email, &subject, &html).await
    }

    // =====================================================
    // Employee lifecycle notifications
    // =====================================================

    pub async fn send_employee_invite_email(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        invite_link: &str,
        school_name: &str,
    ) -> Result<(), AppError> {
        let subject = format!("Welcome to {} — Employee Access", school_name);
        let html =
            email_templates::employee_invite_html(first_name, last_name, invite_link, school_name);
        self.dispatch(email, &subject, &html).await
    }

    pub async fn send_employee_form_assigned_email(
        &self,
        email: &str,
        employee_name: &str,
        form_name: &str,
        due_date: &str,
        dashboard_url: &str,
    ) -> Result<(), AppError> {
        let subject = format!("New form assigned: {}", form_name);
        let html = email_templates::employee_form_assigned_html(
            employee_name,
            form_name,
            due_date,
            dashboard_url,
        );
        self.dispatch(email, &subject, &html).await
    }

    pub async fn send_employee_form_approved_email(
        &self,
        email: &str,
        employee_name: &str,
        form_name: &str,
        notes: &str,
    ) -> Result<(), AppError> {
        let subject = format!("{} has been approved", form_name);
        let html = email_templates::employee_form_approved_html(employee_name, form_name, notes);
        self.dispatch(email, &subject, &html).await
    }

    pub async fn send_employee_form_rejected_email(
        &self,
        email: &str,
        employee_name: &str,
        form_name: &str,
        notes: &str,
    ) -> Result<(), AppError> {
        let subject = format!("Action needed: {} requires updates", form_name);
        let html = email_templates::employee_form_rejected_html(employee_name, form_name, notes);
        self.dispatch(email, &subject, &html).await
    }

    pub async fn send_employee_form_reminder_email(
        &self,
        email: &str,
        employee_name: &str,
        form_name: &str,
        due_date: &str,
        dashboard_url: &str,
    ) -> Result<(), AppError> {
        let subject = format!("Reminder: {} is due soon", form_name);
        let html = email_templates::employee_form_reminder_html(
            employee_name,
            form_name,
            due_date,
            dashboard_url,
        );
        self.dispatch(email, &subject, &html).await
    }
}

/// Resolve the parent dashboard base URL used in CTA buttons.
pub fn parent_dashboard_url() -> String {
    std::env::var("PARENT_DASHBOARD_URL")
        .unwrap_or_else(|_| "https://dev.goddard-web.pages.dev/".to_string())
}
