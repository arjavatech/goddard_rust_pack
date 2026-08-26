use crate::{
    error::{ApiResult, AppError},
    models::email::{
        BulkEmailResponse, ChildAddedNotification, ChildArchivedNotification,
        FormApprovedNotification, FormAssignedNotification, FormRejectedNotification,
        ParentDeactivatedNotification, ParentFormReminder,
    },
    models::document_request::DocumentReminder,
    services::{
        email_provider::{BatchRecipient, EmailProvider, SmtpProvider, ZeptoMailProvider},
        email_templates,
    },
};
use chrono::{NaiveDate, Utc};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Default)]
struct ParentReminderGroup {
    parent_name: String,
    children: BTreeMap<String, ChildReminderGroup>,
}

#[derive(Default)]
struct ChildReminderGroup {
    student_name: String,
    class_name: String,
    forms: BTreeSet<(String, String, bool)>,
}

#[derive(Default)]
struct DocumentReminderGroup {
    recipient_name: String,
    reminders: Vec<DocumentReminder>,
}

pub struct EmailService {
    provider: Arc<dyn EmailProvider>,
}

impl EmailService {
    pub fn new() -> Self {
        let provider: Arc<dyn EmailProvider> =
            if std::env::var("EMAIL_PROVIDER").ok().as_deref() == Some("zeptomail") {
                Arc::new(ZeptoMailProvider::new().expect("Invalid ZeptoMail configuration"))
            } else {
                Arc::new(SmtpProvider::new())
            };
        Self { provider }
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
        Ok(Self::parse_due_date(dd_mm_yyyy)?
            .format("%B %d, %Y")
            .to_string())
    }

    fn parse_due_date(dd_mm_yyyy: &str) -> Result<NaiveDate, AppError> {
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

        Ok(date)
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

    async fn send_form_reminder_email(
        &self,
        reminder: &ParentFormReminder,
    ) -> Result<(), AppError> {
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
            format!(
                "Reminder: {} for {}",
                reminder.form_name, reminder.student_name
            )
        } else {
            format!(
                "Reminder: {} for {} - Due {}",
                reminder.form_name, reminder.student_name, formatted_due_date
            )
        };

        self.dispatch(&reminder.parent_email, &subject, &html_body)
            .await
    }

    pub async fn send_bulk_form_reminders(
        &self,
        reminders: Vec<ParentFormReminder>,
    ) -> ApiResult<BulkEmailResponse> {
        println!(
            "[EmailService] Starting bulk reminder consolidation: {} form rows",
            reminders.len()
        );

        let form_rows = reminders.len();
        let mut parent_groups: BTreeMap<String, ParentReminderGroup> = BTreeMap::new();

        for reminder in reminders {
            let (due_date, is_overdue) = if reminder.due_date.trim().is_empty() {
                ("at your earliest convenience".to_string(), false)
            } else {
                let due_date = Self::parse_due_date(&reminder.due_date)?;
                (
                    due_date.format("%B %d, %Y").to_string(),
                    due_date < Utc::now().date_naive(),
                )
            };
            let student_name = reminder.student_name.trim().to_string();
            let class_name = reminder.class_name.trim().to_string();
            let child_key = format!(
                "{}\u{1f}{}",
                student_name.to_lowercase(),
                class_name.to_lowercase()
            );

            // The current client payload permits a comma-separated value. Each
            // address receives its own consolidated message without changing
            // the API contract.
            for email in reminder.parent_email.split(',') {
                let email = email.trim().to_lowercase();
                if email.is_empty() {
                    continue;
                }

                let parent = parent_groups
                    .entry(email)
                    .or_insert_with(|| ParentReminderGroup {
                        parent_name: reminder.parent_name.trim().to_string(),
                        children: BTreeMap::new(),
                    });
                if parent.parent_name.is_empty() && !reminder.parent_name.trim().is_empty() {
                    parent.parent_name = reminder.parent_name.trim().to_string();
                }

                let child = parent.children.entry(child_key.clone()).or_insert_with(|| {
                    ChildReminderGroup {
                        student_name: student_name.clone(),
                        class_name: class_name.clone(),
                        forms: BTreeSet::new(),
                    }
                });
                child
                    .forms
                    .insert((reminder.form_name.trim().to_string(), due_date.clone(), is_overdue));
            }
        }

        if parent_groups.is_empty() {
            return Err(AppError::Validation(
                "No recipient email addresses provided".to_string(),
            ));
        }

        let total_sent = parent_groups.len();
        let recipients = parent_groups
            .into_iter()
            .map(|(email, parent)| {
                let children_html = Self::render_bulk_reminder_children(&parent.children);
                let child_count = parent.children.len();
                let form_count = parent
                    .children
                    .values()
                    .map(|child| child.forms.len())
                    .sum::<usize>();
                let parent_name = if parent.parent_name.trim().is_empty() {
                    "Parent".to_string()
                } else {
                    parent.parent_name
                };
                let (introduction, closing) = Self::bulk_reminder_copy(child_count, form_count);
                BatchRecipient {
                    address: email,
                    name: parent_name.clone(),
                    merge_info: serde_json::json!({
                        "parent_name": email_templates::html_escape(&parent_name),
                        "introduction": introduction,
                        "children_html": children_html,
                        "closing": closing,
                    }),
                }
            })
            .collect::<Vec<_>>();
        let subject = "Reminder: Forms need your attention".to_string();
        let html = email_templates::bulk_form_reminder_html(
            "{{parent_name}}",
            "{{introduction}}",
            "{{children_html}}",
            "{{closing}}",
            &parent_dashboard_url(),
        );
        self.provider.send_batch(subject, html, recipients).await?;
        let total_failed = 0;
        let failed_emails = Vec::new();
        let message = format!(
            "{} consolidated reminder emails accepted for delivery, covering {} form rows",
            total_sent, form_rows
        );

        println!("[EmailService] Bulk send complete: {}", message);

        Ok(BulkEmailResponse {
            total_sent,
            total_failed,
            failed_emails,
            message,
        })
    }

    pub async fn send_bulk_document_reminders(&self, reminders: Vec<DocumentReminder>) -> ApiResult<BulkEmailResponse> {
        let reminder_rows = reminders.len();
        let mut groups: BTreeMap<String, DocumentReminderGroup> = BTreeMap::new();
        for reminder in reminders {
            let email = reminder.recipient_email.trim().to_lowercase();
            if email.is_empty() { continue; }
            let group = groups.entry(email).or_default();
            if group.recipient_name.is_empty() { group.recipient_name = reminder.recipient_name.trim().to_string(); }
            group.reminders.push(reminder);
        }
        if groups.is_empty() { return Err(AppError::Validation("No recipient email addresses are available for the selected documents".into())); }

        let recipients = groups.into_iter().map(|(email, group)| {
            let name = if group.recipient_name.is_empty() { "Recipient".to_string() } else { group.recipient_name };
            let documents_html = Self::render_document_reminders(&group.reminders);
            let introduction = if group.reminders.len() == 1 { "A document requires your attention." } else { "The following documents require your attention." };
            BatchRecipient { address: email, name: name.clone(), merge_info: serde_json::json!({
                "recipient_name": email_templates::html_escape(&name),
                "introduction": introduction,
                "documents_html": documents_html,
            }) }
        }).collect::<Vec<_>>();
        let total_sent = recipients.len();
        let html = email_templates::bulk_document_reminder_html("{{recipient_name}}", "{{introduction}}", "{{documents_html}}", &parent_dashboard_url());
        self.provider.send_batch("Reminder: Documents need your attention".to_string(), html, recipients).await?;
        Ok(BulkEmailResponse { total_sent, total_failed: 0, failed_emails: Vec::new(), message: format!("{} reminder email(s) accepted for delivery, covering {} document assignment(s)", total_sent, reminder_rows) })
    }

    fn render_document_reminders(reminders: &[DocumentReminder]) -> String {
        let rows = reminders.iter().map(|reminder| {
            let due = reminder.due_date.map(|date| date.format("%B %d, %Y").to_string()).unwrap_or_else(|| "at your earliest convenience".to_string());
            let status = if reminder.rejection_reason.as_deref().unwrap_or("").trim().is_empty() {
                if reminder.is_overdue { "Overdue".to_string() } else { "Pending upload".to_string() }
            } else { "Re-upload required".to_string() };
            let person = if reminder.audience == "student" { format!("{}{}", email_templates::html_escape(&reminder.subject_name), reminder.classroom_name.as_deref().map(|classroom| format!(" · {}", email_templates::html_escape(classroom))).unwrap_or_default()) } else { email_templates::html_escape(&reminder.subject_name) };
            let rejection = reminder.rejection_reason.as_deref().filter(|reason| !reason.trim().is_empty()).map(|reason| format!("<br/><span style=\"color:#b42318;\"><strong>Re-upload instruction:</strong> {}</span>", email_templates::html_escape(reason))).unwrap_or_default();
            format!("<li style=\"margin:0 0 14px;\"><strong>{}</strong><br/><span style=\"color:#555;\">{} · Due {}</span><br/><span style=\"color:#1e4b83;\">{}</span>{}</li>", email_templates::html_escape(&reminder.document_name), person, due, status, rejection)
        }).collect::<Vec<_>>().join("");
        format!("<ul style=\"padding-left:20px;margin:18px 0;\">{}</ul>", rows)
    }

    fn render_bulk_reminder_children(children: &BTreeMap<String, ChildReminderGroup>) -> String {
        children
            .values()
            .map(|child| {
                let forms = child
                    .forms
                    .iter()
                    .map(|(form_name, due_date, is_overdue)| {
                        let overdue = if *is_overdue {
                            r#" <span style="display:inline-block;margin-left:6px;padding:2px 7px;background:#fef2f2;color:#b91c1c;border:1px solid #fecaca;border-radius:10px;font-size:12px;font-weight:bold;">Overdue</span>"#
                        } else {
                            ""
                        };
                        format!(
                            r#"<li style="margin:0 0 8px;color:#333;"><strong>{}</strong> <span style="color:#64748b;">— Due {}</span>{}</li>"#,
                            email_templates::html_escape(form_name),
                            email_templates::html_escape(due_date),
                            overdue,
                        )
                    })
                    .collect::<String>();
                let classroom = if child.class_name.is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<p style="margin:4px 0 10px;color:#64748b;font-size:14px;">Classroom: {}</p>"#,
                        email_templates::html_escape(&child.class_name),
                    )
                };

                format!(
                    r#"<div style="margin:20px 0;padding:16px;background:#f7f9fb;border-left:4px solid #3498db;border-radius:4px;">
                          <p style="margin:0;color:#2c3e50;font-size:16px;"><strong>{}</strong></p>
                          {}
                          <ul style="margin:0;padding-left:20px;">{}</ul>
                        </div>"#,
                    email_templates::html_escape(&child.student_name),
                    classroom,
                    forms,
                )
            })
            .collect()
    }

    fn bulk_reminder_copy(child_count: usize, form_count: usize) -> (&'static str, &'static str) {
        match (child_count, form_count) {
            (1, 1) => (
                "Please complete the following form for your child:",
                "Please complete the required form at your earliest convenience.",
            ),
            (1, _) => (
                "Please complete the following forms for your child:",
                "Please complete the required forms at your earliest convenience.",
            ),
            _ => (
                "Please complete the following forms for your children:",
                "Please complete the required forms for each child at your earliest convenience.",
            ),
        }
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
