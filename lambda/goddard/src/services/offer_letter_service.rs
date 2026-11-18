use std::env;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dao::{AuthDao, EmployeeDao, OfferLetterDao, SchoolDao},
    error::{ApiResult, AppError},
    models::employee::UpdateEmployeeRequest,
    models::notification::{notification_type, CreateNotification},
    models::offer_letter::{
        AdminReviewRequest, CreateOfferLetterRequest, OfferLetter, OfferLetterWebhookPayload,
        UpdateOfferLetterRequest,
    },
    models::school::SchoolFeature,
    services::{EmailService, NotificationService},
};

pub struct OfferLetterService {
    offer_letter_dao: OfferLetterDao,
    employee_dao: EmployeeDao,
    auth_dao: AuthDao,
    school_dao: SchoolDao,
    email_service: Arc<EmailService>,
    notification_service: Arc<NotificationService>,
}

impl OfferLetterService {
    pub fn new(
        offer_letter_dao: OfferLetterDao,
        employee_dao: EmployeeDao,
        auth_dao: AuthDao,
        school_dao: SchoolDao,
        email_service: Arc<EmailService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            offer_letter_dao,
            employee_dao,
            auth_dao,
            school_dao,
            email_service,
            notification_service,
        }
    }

    fn api_base_url() -> String {
        env::var("API_BASE_URL").unwrap_or_else(|_| "https://api.goddard-app.com".to_string())
    }

    fn app_url() -> String {
        env::var("FRONTEND_URL").unwrap_or_else(|_| "https://app.goddard-app.com".to_string())
    }

    async fn ensure_feature_enabled(&self, school_id: Uuid) -> ApiResult<()> {
        self.school_dao.ensure_feature_enabled(school_id, SchoolFeature::OfferLetter).await
    }

    // ─── Create ───────────────────────────────────────────────────────────────

    pub async fn create_offer_letter(
        &self,
        employee_id: Uuid,
        req: CreateOfferLetterRequest,
    ) -> ApiResult<OfferLetter> {
        self.ensure_feature_enabled(req.school_id).await?;

        let employee = self.employee_dao
            .get_employee_by_id(employee_id, req.school_id).await?
            .ok_or_else(|| AppError::NotFound("Employee not found".to_string()))?;

        // Allow pending_offer (new hire flow) OR null offer_status (existing active employee retroactive offer)
        let is_existing_active = employee.offer_status.is_none() && employee.is_active.unwrap_or(false);
        if !is_existing_active && employee.offer_status.as_deref() != Some("pending_offer") {
            return Err(AppError::Validation(
                "Offer letter can only be created for employees in pending_offer status or existing active employees".to_string(),
            ));
        }

        let school = self.school_dao.get_school_by_id(&req.school_id).await?
            .ok_or_else(|| AppError::NotFound("School not found".to_string()))?;
        let fillout_form_id = school.offer_letter_fillout_form_id;

        let offer = self.offer_letter_dao.create(
            req.school_id,
            employee_id,
            &req.first_name,
            &req.last_name,
            &req.email,
            req.phone.as_deref(),
            fillout_form_id.as_deref(),
            req.custom_fields.as_ref(),
        ).await?;

        self.employee_dao.set_offer_status(employee_id, req.school_id, "draft").await?;

        Ok(offer)
    }

    // ─── Update ───────────────────────────────────────────────────────────────

    pub async fn update_offer_letter(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
        req: UpdateOfferLetterRequest,
    ) -> ApiResult<OfferLetter> {
        self.ensure_feature_enabled(school_id).await?;

        let updated = self.offer_letter_dao.update(
            employee_id,
            school_id,
            req.first_name.as_deref(),
            req.last_name.as_deref(),
            req.phone.as_deref(),
            None,
            req.custom_fields.as_ref(),
        ).await?;

        // Sync first_name / last_name back to users table
        if req.first_name.is_some() || req.last_name.is_some() {
            if let Ok(Some(emp)) = self.employee_dao.get_employee_by_id(employee_id, school_id).await {
                let first = req.first_name.as_deref().unwrap_or(&updated.first_name);
                let last = req.last_name.as_deref().unwrap_or(&updated.last_name);
                let _ = self.auth_dao.update_user_names(emp.user_id, first, last).await;
            }
        }

        // Sync phone + joined_on back to employees table
        let joined_on = extract_joining_date(req.custom_fields.as_ref());
        if req.phone.is_some() || joined_on.is_some() {
            let emp_req = UpdateEmployeeRequest {
                phone: req.phone.clone(),
                address: None,
                employee_type: None,
                joined_on,
            };
            let _ = self.employee_dao.update_employee(employee_id, school_id, &emp_req).await;
        }

        Ok(updated)
    }

    // ─── Send ─────────────────────────────────────────────────────────────────

    pub async fn send_offer_letter(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<OfferLetter> {
        self.ensure_feature_enabled(school_id).await?;

        let offer = self.offer_letter_dao.get_by_employee_id(employee_id, school_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))?;

        if offer.fillout_form_id.is_none() {
            return Err(AppError::Validation(
                "No Fillout form configured for offer letters. Please configure it in Employee Forms settings.".to_string(),
            ));
        }

        let school_name = self.school_dao.get_school_name(&school_id).await
            .map_err(|_| AppError::NotFound("School not found".to_string()))?;

        let invite_token = self.auth_dao
            .create_invite_token(&offer.email, "Employee", school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to create invite token for offer letter: {}", e);
                Uuid::nil()
            });

        let invite_link = format!(
            "{}/enrollments/activate/{}",
            Self::api_base_url(),
            invite_token
        );

        let prefill_url = build_prefill_url(&offer, invite_token);

        let updated_offer = self.offer_letter_dao.set_sent(employee_id, school_id, &prefill_url).await?;

        self.employee_dao.set_offer_status(employee_id, school_id, "offer_sent").await?;

        if invite_token != Uuid::nil() {
            let _ = self.email_service.send_offer_letter_email(
                &offer.email,
                &offer.first_name,
                &offer.last_name,
                &school_name,
                "Employee",
                &invite_link,
            ).await.map_err(|e| {
                tracing::warn!("Failed to send offer letter email to {}: {}", offer.email, e);
            });
        }

        if let Ok(Some(emp)) = self.employee_dao.get_employee_by_id(employee_id, school_id).await {
            let body = format!("Your offer letter from {} is ready to review and sign.", school_name);
            self.notification_service.notify_user(emp.user_id, CreateNotification {
                school_id,
                notification_type: notification_type::OFFER_LETTER_SENT.to_string(),
                title: "Your Offer Letter is Ready".to_string(),
                body,
                related_entity_id: Some(updated_offer.id),
                related_entity_type: Some("offer_letter".to_string()),
                action_url: Some("/employee/offer-letter".to_string()),
            }).await;
        }

        Ok(updated_offer)
    }

    // ─── Resend ───────────────────────────────────────────────────────────────

    pub async fn resend_offer_letter(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<OfferLetter> {
        self.ensure_feature_enabled(school_id).await?;

        let offer = self.offer_letter_dao.get_by_employee_id(employee_id, school_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))?;

        let school_name = self.school_dao.get_school_name(&school_id).await
            .map_err(|_| AppError::NotFound("School not found".to_string()))?;

        let login_url = Self::app_url();
        let prefill_url = build_prefill_url(&offer, Uuid::nil());

        let updated_offer = self.offer_letter_dao.set_reopened(employee_id, school_id, &prefill_url).await?;
        self.employee_dao.set_offer_status(employee_id, school_id, "offer_sent").await?;

        let _ = self.email_service.send_offer_letter_resend_email(
            &offer.email,
            &offer.first_name,
            &offer.last_name,
            &school_name,
            &login_url,
        ).await;

        if let Ok(Some(emp)) = self.employee_dao.get_employee_by_id(offer.employee_id, offer.school_id).await {
            let body = format!("Your offer letter from {} has been resent.", school_name);
            self.notification_service.notify_user(emp.user_id, CreateNotification {
                school_id: offer.school_id,
                notification_type: notification_type::OFFER_LETTER_SENT.to_string(),
                title: "Offer Letter Resent".to_string(),
                body,
                related_entity_id: Some(updated_offer.id),
                related_entity_type: Some("offer_letter".to_string()),
                action_url: Some("/employee/offer-letter".to_string()),
            }).await;
        }

        Ok(updated_offer)
    }

    // ─── Webhook ──────────────────────────────────────────────────────────────

    pub async fn handle_fillout_webhook(&self, payload: OfferLetterWebhookPayload) -> ApiResult<OfferLetter> {
        let offer = self.offer_letter_dao.get_by_id_no_school(payload.offer_letter_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))?;

        // Idempotency: skip re-processing if this submission was already stored.
        // Exception: allow re-processing when offer was rejected — employee re-fills using the
        // same edit link, which Fillout may deliver with the same submission ID.
        let is_same_submission = offer.fillout_submission_id.as_deref()
            == Some(payload.fillout_submission_id.as_str());
        let is_rejected = offer.status == "offer_rejected";
        if is_same_submission && !is_rejected {
            return Ok(offer);
        }

        // IN_PROGRESS = employee opened the Fillout form → mark as viewed
        let is_in_progress = payload.form_status.as_deref()
            .map(|s| s.trim().eq_ignore_ascii_case("in_progress"))
            .unwrap_or(false);

        if is_in_progress {
            return Ok(offer);
        }

        // COMPLETED = employee submitted the form = employee_signed
        let accepted = payload.accepted.unwrap_or(true);

        let updated = self.offer_letter_dao.set_fillout_submission(
            payload.offer_letter_id,
            offer.school_id,
            &payload.fillout_submission_id,
            payload.pdf_link.as_deref(),
            payload.edit_link.as_deref(),
            payload.form_data.as_ref(),
            accepted,
            payload.rejection_reason.as_deref(),
            payload.submitted_at,
        ).await?;

        let new_status = if accepted { "employee_signed" } else { "offer_rejected" };
        self.employee_dao.set_offer_status(offer.employee_id, offer.school_id, new_status).await?;

        let (notif_type, notif_title) = if accepted {
            (notification_type::OFFER_LETTER_ACCEPTED, "Offer Letter Signed")
        } else {
            (notification_type::OFFER_LETTER_REJECTED, "Offer Letter Rejected")
        };
        let verb = if accepted { "signed" } else { "rejected" };
        let notif_body = format!("{} {} has {} their offer letter.", offer.first_name, offer.last_name, verb);
        self.notification_service.notify_school_admins(CreateNotification {
            school_id: offer.school_id,
            notification_type: notif_type.to_string(),
            title: notif_title.to_string(),
            body: notif_body,
            related_entity_id: Some(updated.id),
            related_entity_type: Some("offer_letter".to_string()),
            action_url: Some("/admin/offer-letters".to_string()),
        }, None).await;

        Ok(updated)
    }

    // ─── Admin review ─────────────────────────────────────────────────────────

    pub async fn admin_review(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
        req: AdminReviewRequest,
        reviewer_id: Uuid,
    ) -> ApiResult<OfferLetter> {
        self.ensure_feature_enabled(school_id).await?;

        let updated = self.offer_letter_dao.set_admin_review(
            employee_id,
            school_id,
            reviewer_id,
            req.approved,
            req.rejection_reason.as_deref(),
        ).await?;

        if req.approved {
            self.employee_dao.activate_employee_offer_approved(employee_id, school_id).await?;
        } else {
            self.employee_dao.set_offer_status(employee_id, school_id, "offer_rejected").await?;
        }

        Ok(updated)
    }

    // ─── Mark viewed ─────────────────────────────────────────────────────────

    pub async fn mark_viewed(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<OfferLetter> {
        let offer = self.offer_letter_dao.get_by_employee_id(employee_id, school_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))?;

        if offer.status == "offer_sent" {
            self.offer_letter_dao.set_viewed(offer.id, offer.school_id).await?;
            let _ = self.employee_dao.set_offer_status(employee_id, school_id, "viewed").await;
        }

        self.offer_letter_dao.get_by_employee_id(employee_id, school_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))
    }

    // ─── Get ──────────────────────────────────────────────────────────────────

    pub async fn get_offer_letter(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<OfferLetter> {
        self.offer_letter_dao.get_by_employee_id(employee_id, school_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))
    }

    // ─── List ─────────────────────────────────────────────────────────────────

    pub async fn list_for_school(&self, school_id: Uuid) -> ApiResult<Vec<OfferLetter>> {
        self.ensure_feature_enabled(school_id).await?;
        self.offer_letter_dao.list_by_school(school_id).await
    }

    // ─── History ──────────────────────────────────────────────────────────────

    pub async fn get_offer_letter_history(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<Vec<OfferLetter>> {
        self.offer_letter_dao.list_history_by_employee(employee_id, school_id).await
    }

}

pub fn build_prefill_url(offer: &OfferLetter, _invite_token: Uuid) -> String {
    let form_id = match &offer.fillout_form_id {
        Some(id) => id.clone(),
        None => return String::new(),
    };

    let mut params = vec![
        ("offer_letter_id".to_string(), offer.id.to_string()),
        ("firstName".to_string(), offer.first_name.clone()),
        ("lastName".to_string(), offer.last_name.clone()),
        ("email".to_string(), offer.email.clone()),
    ];

    if let Some(ref phone) = offer.phone {
        params.push(("phone".to_string(), phone.clone()));
    }

    if let Some(serde_json::Value::Object(fields)) = &offer.custom_fields {
        for (k, v) in fields {
            let s = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
            params.push((k.clone(), s));
        }
    }

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding(v)))
        .collect::<Vec<_>>()
        .join("&");

    let sep = if form_id.contains('?') { '&' } else { '?' };
    format!("{}{}{}", form_id, sep, query)
}

fn extract_joining_date(custom_fields: Option<&serde_json::Value>) -> Option<chrono::NaiveDate> {
    let obj = custom_fields?.as_object()?;
    let date_keys = ["joining_date", "joiningDate", "joined_on", "joinedOn", "start_date", "startDate"];
    for key in &date_keys {
        if let Some(val) = obj.get(*key) {
            if let Some(s) = val.as_str() {
                if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    return Some(d);
                }
            }
        }
    }
    None
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c]
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}
