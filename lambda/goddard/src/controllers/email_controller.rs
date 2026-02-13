use axum::{
    extract::State,
    response::IntoResponse,
    Json,
    Extension,
};
use std::sync::Arc;

use crate::{
    services::EmailService,
    models::{
        email::{BulkEmailRequest, BulkEmailResponse},
        schema::UserRole,
    },
    utils::ResponseUtils,
    error::AppError,
    middleware::auth::AuthContext,
};

/// POST /emails/bulk-form-reminders
/// Send bulk form reminder emails to parents (API Key protected - Admin/SuperAdmin only)
pub async fn send_bulk_form_reminders(
    Extension(auth): Extension<AuthContext>,
    State(email_service): State<Arc<EmailService>>,
    Json(payload): Json<BulkEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!("[EmailController] Bulk email request from user: {}", auth.user_id);
    println!("[EmailController] School ID: {}", payload.school_id);
    println!("[EmailController] Number of emails: {}", payload.reminders.len());

    // Validation: Check role is Admin or SuperAdmin
    if !matches!(auth.role, UserRole::Admin | UserRole::SuperAdmin) {
        println!("[EmailController] Unauthorized: role={:?}", auth.role);
        return Err(AppError::Authorization(
            "Only Admin and SuperAdmin can send bulk emails".to_string()
        ));
    }

    // Validation: Check school authorization
    // SuperAdmin can email any school, Admin must match their school_id
    if matches!(auth.role, UserRole::Admin) {
        if auth.school_id != payload.school_id {
            println!("[EmailController] Admin trying to access different school: user_school={}, requested={}",
                auth.school_id, payload.school_id);
            return Err(AppError::Authorization(
                "Admin can only send emails for their own school".to_string()
            ));
        }
        println!("[EmailController] Admin authorized for their school");
    } else {
        // SuperAdmin can access any school
        println!("[EmailController] SuperAdmin accessing school: {}", payload.school_id);
    }

    // Validation: Check batch size limit (max 100)
    if payload.reminders.len() > 100 {
        println!("[EmailController] Batch size exceeded: {}", payload.reminders.len());
        return Err(AppError::Validation(
            format!("Maximum 100 emails per request. Received: {}", payload.reminders.len())
        ));
    }

    // Validation: Check for empty batch
    if payload.reminders.is_empty() {
        println!("[EmailController] Empty batch received");
        return Err(AppError::Validation("No email reminders provided".to_string()));
    }

    // Send emails
    let response = email_service.send_bulk_form_reminders(payload.reminders).await?;

    println!("[EmailController] Bulk send complete: sent={}, failed={}",
        response.total_sent, response.total_failed);

    Ok(ResponseUtils::success(response))
}
