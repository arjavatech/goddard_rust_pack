use std::env;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dao::{AuthDao, EmployeeDao, OfferLetterDao, SchoolDao},
    error::{ApiResult, AppError},
    models::offer_letter::{
        AdminReviewRequest, CreateOfferLetterRequest, OfferLetter, OfferLetterWebhookPayload,
        UpdateOfferLetterRequest,
    },
    models::school::SchoolFeature,
    services::EmailService,
};

pub struct OfferLetterService {
    offer_letter_dao: OfferLetterDao,
    employee_dao: EmployeeDao,
    auth_dao: AuthDao,
    school_dao: SchoolDao,
    email_service: Arc<EmailService>,
}

impl OfferLetterService {
    pub fn new(
        offer_letter_dao: OfferLetterDao,
        employee_dao: EmployeeDao,
        auth_dao: AuthDao,
        school_dao: SchoolDao,
        email_service: Arc<EmailService>,
    ) -> Self {
        Self {
            offer_letter_dao,
            employee_dao,
            auth_dao,
            school_dao,
            email_service,
        }
    }

    fn api_base_url() -> String {
        env::var("API_BASE_URL").unwrap_or_else(|_| "https://api.goddard-app.com".to_string())
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
            &req.role,
            req.salary,
            req.salary_currency.as_deref(),
            req.joining_date,
            req.additional_notes.as_deref(),
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

        self.offer_letter_dao.update(
            employee_id,
            school_id,
            req.first_name.as_deref(),
            req.last_name.as_deref(),
            req.phone.as_deref(),
            req.role.as_deref(),
            req.salary,
            req.salary_currency.as_deref(),
            req.joining_date,
            req.additional_notes.as_deref(),
            None,
            req.custom_fields.as_ref(),
        ).await
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

        let prefill_url = Self::build_prefill_url(&offer, invite_token);

        let updated_offer = self.offer_letter_dao.set_sent(employee_id, school_id, &prefill_url).await?;

        self.employee_dao.set_offer_status(employee_id, school_id, "offer_sent").await?;

        if invite_token != Uuid::nil() {
            let _ = self.email_service.send_offer_letter_email(
                &offer.email,
                &offer.first_name,
                &offer.last_name,
                &school_name,
                &invite_link,
                &offer.role,
            ).await.map_err(|e| {
                tracing::warn!("Failed to send offer letter email to {}: {}", offer.email, e);
            });
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

        let invite_token = self.auth_dao
            .create_invite_token(&offer.email, "Employee", school_id)
            .await
            .unwrap_or(Uuid::nil());

        let invite_link = format!(
            "{}/enrollments/activate/{}",
            Self::api_base_url(),
            invite_token
        );

        let prefill_url = Self::build_prefill_url(&offer, invite_token);
        let updated_offer = self.offer_letter_dao.set_sent(employee_id, school_id, &prefill_url).await?;

        if invite_token != Uuid::nil() {
            let _ = self.email_service.send_offer_letter_email(
                &offer.email,
                &offer.first_name,
                &offer.last_name,
                &school_name,
                &invite_link,
                &offer.role,
            ).await;
        }

        Ok(updated_offer)
    }

    // ─── Webhook ──────────────────────────────────────────────────────────────

    pub async fn handle_fillout_webhook(&self, payload: OfferLetterWebhookPayload) -> ApiResult<OfferLetter> {
        let offer = self.offer_letter_dao.get_by_id_no_school(payload.offer_letter_id).await?
            .ok_or_else(|| AppError::NotFound("Offer letter not found".to_string()))?;

        let updated = self.offer_letter_dao.set_fillout_submission(
            payload.offer_letter_id,
            &payload.fillout_submission_id,
            payload.pdf_link.as_deref(),
            payload.edit_link.as_deref(),
            payload.form_data.as_ref(),
            payload.accepted,
            payload.rejection_reason.as_deref(),
        ).await?;

        let new_status = if payload.accepted { "offer_accepted" } else { "offer_rejected" };
        self.employee_dao.set_offer_status(offer.employee_id, offer.school_id, new_status).await?;

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

    // ─── Helpers ──────────────────────────────────────────────────────────────

    fn build_prefill_url(offer: &OfferLetter, _invite_token: Uuid) -> String {
        let form_id = match &offer.fillout_form_id {
            Some(id) => id.clone(),
            None => return String::new(),
        };

        let mut params = vec![
            ("offer_letter_id".to_string(), offer.id.to_string()),
            ("firstName".to_string(), offer.first_name.clone()),
            ("lastName".to_string(), offer.last_name.clone()),
            ("email".to_string(), offer.email.clone()),
            ("role".to_string(), offer.role.clone()),
        ];

        if let Some(ref phone) = offer.phone {
            params.push(("phone".to_string(), phone.clone()));
        }
        if let Some(salary) = offer.salary {
            params.push(("salary".to_string(), salary.to_string()));
        }
        if let Some(ref currency) = offer.salary_currency {
            params.push(("salaryCurrency".to_string(), currency.clone()));
        }
        if let Some(ref date) = offer.joining_date {
            params.push(("joiningDate".to_string(), date.to_string()));
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

        format!("https://fillout.com/{}?{}", form_id, query)
    }
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
