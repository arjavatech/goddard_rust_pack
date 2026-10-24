use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use std::sync::Arc;

use crate::{
    dao::HelpCenterDao,
    error::AppError,
    middleware::auth::AuthContext,
    models::help_center::SubmitHelpMessageRequest,
    services::{email_templates::help_center_message_html, EmailService},
};
use crate::dao::SchoolDao;

pub async fn submit_help_message(
    State((help_dao, email_svc, school_dao)): State<(Arc<HelpCenterDao>, Arc<EmailService>, SchoolDao)>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SubmitHelpMessageRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.sender_name.trim().is_empty()
        || req.sender_email.trim().is_empty()
        || req.subject.trim().is_empty()
        || req.message.trim().is_empty()
    {
        return Err(AppError::Validation("All fields are required".into()));
    }

    // Resolve recipient: SuperAdmin user → school settings.mail → hard fallback
    let to_email = match help_dao.get_school_superadmin_email(auth.school_id).await? {
        Some(email) if !email.is_empty() => email,
        _ => match help_dao.get_school_contact_email(auth.school_id).await? {
            Some(email) => email,
            None => "support@arjavatech.com".to_string(),
        },
    };

    let school_name = school_dao
        .get_school_name(&auth.school_id)
        .await
        .unwrap_or_else(|_| "The Goddard School".to_string());

    let sender_role = format!("{:?}", auth.role);

    let msg = help_dao
        .create_message(
            auth.school_id,
            Some(auth.user_id),
            &sender_role,
            &req,
            &to_email,
            &req.sender_email,
        )
        .await?;

    let html = help_center_message_html(
        &req.sender_name,
        &req.sender_email,
        &sender_role,
        &req.subject,
        &req.message,
        &school_name,
    );
    let subject_line = format!("Help Request from {}: {}", req.sender_name, req.subject);
    if let Err(e) = email_svc.dispatch(&to_email, &subject_line, &html).await {
        eprintln!("[HelpCenter] Email send failed (message stored): {:?}", e);
    }

    Ok((StatusCode::CREATED, Json(msg)))
}
