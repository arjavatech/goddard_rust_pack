use axum::{extract::State, response::IntoResponse, Extension, Json};
use std::sync::Arc;

use crate::{
    error::AppError,
    middleware::auth::AuthContext,
    models::{email::BulkEmailRequest, schema::UserRole},
    services::EmailService,
    utils::ResponseUtils,
};

/// POST /emails/bulk-form-reminders
/// Send bulk form reminder emails to parents (API Key protected - Admin/SuperAdmin only)
pub async fn send_bulk_form_reminders(
    Extension(auth): Extension<AuthContext>,
    State(email_service): State<Arc<EmailService>>,
    Json(payload): Json<BulkEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!(
        "[EmailController] Bulk email request from user: {}",
        auth.user_id
    );
    println!("[EmailController] School ID: {}", payload.school_id);
    println!(
        "[EmailController] Number of emails: {}",
        payload.reminders.len()
    );

    // Validation: Check role is Admin or SuperAdmin
    if !matches!(auth.role, UserRole::Admin | UserRole::SuperAdmin) {
        println!("[EmailController] Unauthorized: role={:?}", auth.role);
        return Err(AppError::Authorization(
            "Only Admin and SuperAdmin can send bulk emails".to_string(),
        ));
    }

    // Validation: Check school authorization
    // SuperAdmin can email any school, Admin must match their school_id
    if matches!(auth.role, UserRole::Admin) {
        if auth.school_id != payload.school_id {
            println!("[EmailController] Admin trying to access different school: user_school={}, requested={}",
                auth.school_id, payload.school_id);
            return Err(AppError::Authorization(
                "Admin can only send emails for their own school".to_string(),
            ));
        }
        println!("[EmailController] Admin authorized for their school");
    } else {
        // SuperAdmin can access any school
        println!(
            "[EmailController] SuperAdmin accessing school: {}",
            payload.school_id
        );
    }

    // Limit submitted form rows. Recipients are consolidated by email below and
    // delivered by the provider in its supported recipient batch size.
    if payload.reminders.len() > 300 {
        println!(
            "[EmailController] Batch size exceeded: {}",
            payload.reminders.len()
        );
        return Err(AppError::Validation(format!(
            "Maximum 300 form reminders per request. Received: {}",
            payload.reminders.len()
        )));
    }

    // Validation: Check for empty batch
    if payload.reminders.is_empty() {
        println!("[EmailController] Empty batch received");
        return Err(AppError::Validation(
            "No email reminders provided".to_string(),
        ));
    }

    // A parent may have multiple forms and children in this request. The email
    // service consolidates those rows into one message per parent email.
    let response = email_service
        .send_bulk_form_reminders(payload.reminders)
        .await?;

    println!(
        "[EmailController] Bulk send complete: sent={}, failed={}",
        response.total_sent, response.total_failed
    );

    Ok(ResponseUtils::success(response))
}
