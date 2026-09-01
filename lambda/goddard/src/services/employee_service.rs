use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dao::school_dao::SchoolDao,
    dao::{
        AuthDao, EmployeeDao, EmployeeFormAssignmentDao, EmployeeFormSubmissionDao,
        EmployeeFormTemplateDao,
    },
    error::{ApiResult, AppError},
    models::document_request::{
        CompleteUploadRequest, FileAccessResponse, UploadIntentRequest, UploadIntentResponse,
    },
    models::employee::{
        AssignEmployeeFormRequest, AssignEmployeeFormToSchoolRequest,
        AssignEmployeeFormToSchoolResponse, BulkCreateEmployeesRequest,
        BulkCreateEmployeesResponse, BulkCreatedEmployee, BulkEmployeeFormReminderRequest,
        BulkEmployeeReminderResponse, CreateEmployeeFormTemplateRequest, Employee,
        EmployeeFormAssignment, EmployeeFormAssignmentWithTemplate, EmployeeFormSubmission,
        EmployeeFormTemplate, EmployeeInviteRequest, EmployeeInviteResponse, EmployeeWithUser,
        ResendEmployeeInviteResponse, ReviewEmployeeFormRequest, UpdateEmployeeFormTemplateRequest,
        UpdateEmployeeRequest,
    },
    models::form_review_queue::{EmployeeFormReviewQueueItem, FormReviewQueueQuery},
    services::{supabase_client::UserMetadata, EmailService, SupabaseClient, TapTimeMappingService, UploadService},
};

pub struct EmployeeService {
    employee_dao: EmployeeDao,
    employee_form_template_dao: EmployeeFormTemplateDao,
    employee_form_assignment_dao: EmployeeFormAssignmentDao,
    employee_form_submission_dao: EmployeeFormSubmissionDao,
    auth_dao: AuthDao,
    school_dao: SchoolDao,
    supabase_client: SupabaseClient,
    email_service: Arc<EmailService>,
    upload_service: Arc<UploadService>,
    taptime_mapping_service: Arc<TapTimeMappingService>,
}

impl EmployeeService {
    pub fn new(
        employee_dao: EmployeeDao,
        employee_form_template_dao: EmployeeFormTemplateDao,
        employee_form_assignment_dao: EmployeeFormAssignmentDao,
        employee_form_submission_dao: EmployeeFormSubmissionDao,
        auth_dao: AuthDao,
        school_dao: SchoolDao,
        supabase_client: SupabaseClient,
        email_service: Arc<EmailService>,
        upload_service: Arc<UploadService>,
        taptime_mapping_service: Arc<TapTimeMappingService>,
    ) -> Self {
        Self {
            employee_dao,
            employee_form_template_dao,
            employee_form_assignment_dao,
            employee_form_submission_dao,
            auth_dao,
            school_dao,
            supabase_client,
            email_service,
            upload_service,
            taptime_mapping_service,
        }
    }

    fn employee_dashboard_url() -> String {
        let base = env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "https://dev.goddard-web.pages.dev".to_string());
        format!("{}/employee/dashboard", base)
    }

    fn api_base_url() -> String {
        env::var("API_BASE_URL").unwrap_or_else(|_| "https://api.goddard-app.com".to_string())
    }

    // ─── Employee CRUD ─────────────────────────────────────────────────────────

    pub async fn invite_employee(
        &self,
        req: EmployeeInviteRequest,
    ) -> ApiResult<EmployeeInviteResponse> {
        // Validate email basic format
        if req.email.trim().is_empty() || !req.email.contains('@') {
            return Err(AppError::Validation("Invalid email address".to_string()));
        }
        if req.phone.as_deref().map(str::trim).filter(|value| !value.is_empty()).is_none() {
            return Err(AppError::Validation("A phone number is required for a TapTime employee".into()));
        }

        let school_name = self
            .school_dao
            .get_school_name(&req.school_id)
            .await
            .map_err(|_| AppError::NotFound("School not found".to_string()))?;

        // Create user in Supabase (no email sent at this stage)
        let metadata = UserMetadata::new(
            Some(req.school_id),
            Some(req.first_name.clone()),
            Some(req.last_name.clone()),
            Some("Employee".to_string()),
            req.phone.clone(),
            Some(true),
        )
        .with_school_name(school_name.clone());

        let user_id = match self
            .supabase_client
            .create_user_only_in_supabase(&req.email, metadata)
            .await
        {
            Ok(id_str) => Uuid::parse_str(&id_str)
                .map_err(|_| AppError::Internal("Invalid user ID from Supabase".to_string()))?,
            Err(AppError::Conflict(_)) => {
                // User already exists in Supabase/public.users.
                // Check if the employee record was also created (full success on a prior attempt).
                if self
                    .employee_dao
                    .get_employee_by_email_and_school(&req.email, req.school_id)
                    .await?
                    .is_some()
                {
                    return Err(AppError::Conflict(format!(
                        "Employee with email {} already exists in this school",
                        req.email
                    )));
                }
                // Partial failure recovery: user row exists but employee row was never committed.
                // Look up the existing user_id and fall through to create the employee record.
                self.auth_dao.get_user_id_by_email(&req.email).await?
            }
            Err(e) => return Err(e),
        };

        // Create employee record
        let employee = self
            .employee_dao
            .create_employee(
                user_id,
                req.school_id,
                req.phone.as_deref(),
                req.address.as_deref(),
                req.employee_type.as_deref(),
                req.joined_on,
            )
            .await?;

        // Create 7-day invite token
        let invite_token = self
            .auth_dao
            .create_invite_token(&req.email, "Employee", req.school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", req.email, e);
                Uuid::nil()
            });

        let invite_id = invite_token;

        // Send invite email (non-fatal)
        let invite_link = format!(
            "{}/enrollments/activate/{}",
            Self::api_base_url(),
            invite_token
        );
        let email_sent = if invite_token != Uuid::nil() {
            self.email_service
                .send_employee_invite_email(
                    &req.email,
                    &req.first_name,
                    &req.last_name,
                    &invite_link,
                    &school_name,
                )
                .await
                .map(|_| true)
                .unwrap_or_else(|e| {
                    tracing::warn!("Employee invite email failed for {}: {}", req.email, e);
                    false
                })
        } else {
            false
        };

        self.reconcile_taptime_user(req.school_id, user_id).await;

        Ok(EmployeeInviteResponse {
            employee_id: employee.id,
            user_id,
            invite_id,
            email_sent,
            message: if email_sent {
                "Employee invited successfully. Invitation email sent.".to_string()
            } else {
                "Employee record created. Invitation email could not be sent — please resend manually.".to_string()
            },
        })
    }

    pub async fn resend_employee_invite(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<ResendEmployeeInviteResponse> {
        let employee = self
            .employee_dao
            .get_employee_by_id(employee_id, school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee not found".to_string()))?;
        if employee.is_verified == Some(true) {
            return Err(AppError::Conflict(
                "Employee has already completed password setup".to_string(),
            ));
        }
        let school_name = self
            .school_dao
            .get_school_name(&school_id)
            .await
            .map_err(|_| AppError::NotFound("School not found".to_string()))?;
        let token = self
            .auth_dao
            .create_invite_token(&employee.email, "Employee", school_id)
            .await?;
        let link = format!("{}/enrollments/activate/{}", Self::api_base_url(), token);
        let email_sent = match self
            .email_service
            .send_employee_invite_email(
                &employee.email,
                &employee.first_name,
                &employee.last_name,
                &link,
                &school_name,
            )
            .await
        {
            Ok(_) => true,
            Err(error) => {
                tracing::error!(
                    "Employee resend invite email failed for {}: {}",
                    employee.email,
                    error
                );
                false
            }
        };
        Ok(ResendEmployeeInviteResponse {
            employee_id,
            email_sent,
            message: if email_sent {
                "Employee invitation resent".to_string()
            } else {
                "Employee invitation was created but email delivery failed".to_string()
            },
        })
    }

    pub async fn bulk_create_employees(
        &self,
        req: BulkCreateEmployeesRequest,
    ) -> ApiResult<BulkCreateEmployeesResponse> {
        const DEFAULT_EMPLOYEE_PASSWORD: &str = "Keller@2026";

        if req.employees.is_empty() {
            return Err(AppError::Validation(
                "employees must contain at least one employee".to_string(),
            ));
        }

        self.school_dao
            .get_school_name(&req.school_id)
            .await
            .map_err(|_| AppError::NotFound("School not found".to_string()))?;

        let mut submitted_emails = HashSet::new();
        for (index, employee) in req.employees.iter().enumerate() {
            let row = index + 1;
            if employee.first_name.trim().is_empty() {
                return Err(AppError::Validation(format!(
                    "employees[{}].first_name is required",
                    row
                )));
            }
            if employee.last_name.trim().is_empty() {
                return Err(AppError::Validation(format!(
                    "employees[{}].last_name is required",
                    row
                )));
            }
            if employee.phone_number.as_deref().map(str::trim).filter(|value| !value.is_empty()).is_none() {
                return Err(AppError::Validation(format!(
                    "employees[{}].phone_number is required for TapTime",
                    row
                )));
            }
            let email = employee.email.trim();
            if email.is_empty() || !email.contains('@') || !email.contains('.') {
                return Err(AppError::Validation(format!(
                    "employees[{}].email is invalid",
                    row
                )));
            }

            let normalized_email = email.to_lowercase();
            if !submitted_emails.insert(normalized_email) {
                return Err(AppError::Conflict(format!(
                    "Duplicate email in request: {}",
                    email
                )));
            }
            if self.auth_dao.user_exists_by_email(email).await? {
                return Err(AppError::Conflict(format!(
                    "User with email {} already exists",
                    email
                )));
            }
        }

        let mut created_user_ids = Vec::new();
        let mut created_employees = Vec::with_capacity(req.employees.len());
        for employee_input in req.employees {
            let email = employee_input.email.trim().to_string();
            let metadata = UserMetadata::new(
                Some(req.school_id),
                Some(employee_input.first_name.trim().to_string()),
                Some(employee_input.last_name.trim().to_string()),
                Some("Employee".to_string()),
                employee_input.phone_number.clone(),
                Some(true),
            );

            let user_id = match self
                .supabase_client
                .create_user_with_password_in_supabase(&email, DEFAULT_EMPLOYEE_PASSWORD, metadata)
                .await
            {
                Ok(id) => match Uuid::parse_str(&id) {
                    Ok(id) => id,
                    Err(_) => {
                        self.cleanup_bulk_users(&created_user_ids).await;
                        return Err(AppError::Internal(
                            "Invalid user ID returned by Supabase".to_string(),
                        ));
                    }
                },
                Err(error) => {
                    self.cleanup_bulk_users(&created_user_ids).await;
                    return Err(error);
                }
            };
            created_user_ids.push(user_id);

            let employee = match self
                .employee_dao
                .create_employee(
                    user_id,
                    req.school_id,
                    employee_input.phone_number.as_deref(),
                    None,
                    None,
                    None,
                )
                .await
            {
                Ok(employee) => employee,
                Err(error) => {
                    self.cleanup_bulk_users(&created_user_ids).await;
                    return Err(error);
                }
            };
            created_employees.push(BulkCreatedEmployee {
                employee_id: employee.id,
                user_id,
                email,
            });
        }

        Ok(BulkCreateEmployeesResponse {
            school_id: req.school_id,
            created_count: created_employees.len(),
            employees: created_employees,
        })
    }

    async fn cleanup_bulk_users(&self, user_ids: &[Uuid]) {
        for user_id in user_ids.iter().rev() {
            if let Err(error) = self.auth_dao.delete_user_by_id(*user_id).await {
                tracing::error!(
                    "Failed to remove public user {} after bulk employee create failure: {}",
                    user_id,
                    error
                );
            }
            if let Err(error) = self.supabase_client.delete_user_by_id(*user_id).await {
                tracing::error!(
                    "Failed to remove auth user {} after bulk employee create failure: {}",
                    user_id,
                    error
                );
            }
        }
    }

    pub async fn get_current_employee(
        &self,
        user_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<EmployeeWithUser> {
        self.employee_dao
            .get_employee_by_user_id_with_user(user_id, school_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound("Employee record not found for current user".to_string())
            })
    }

    pub async fn get_employees(&self, school_id: Uuid) -> ApiResult<Vec<EmployeeWithUser>> {
        self.employee_dao.get_employees_by_school(school_id).await
    }

    pub async fn get_employee_by_id(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<EmployeeWithUser> {
        self.employee_dao
            .get_employee_by_id(employee_id, school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee not found".to_string()))
    }

    pub async fn update_employee(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
        req: UpdateEmployeeRequest,
    ) -> ApiResult<Employee> {
        let employee = self.employee_dao
            .update_employee(employee_id, school_id, &req)
            .await?;
        self.reconcile_taptime_user(school_id, employee.user_id).await;
        Ok(employee)
    }

    async fn reconcile_taptime_user(&self, school_id: Uuid, user_id: Uuid) {
        if let Err(error) = self.taptime_mapping_service.reconcile_user_email(school_id, user_id).await {
            tracing::warn!(%school_id, %user_id, error = %error, "TapTime email reconciliation deferred after employee change");
        }
    }

    pub async fn deactivate_employee(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<()> {
        self.employee_dao
            .deactivate_employee(employee_id, school_id)
            .await?;
        Ok(())
    }

    pub async fn activate_employee(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<()> {
        self.employee_dao
            .activate_employee(employee_id, school_id)
            .await?;
        Ok(())
    }

    // ─── Employee Form Templates ────────────────────────────────────────────────

    pub async fn create_form_template(
        &self,
        req: CreateEmployeeFormTemplateRequest,
    ) -> ApiResult<EmployeeFormTemplate> {
        self.employee_form_template_dao.create_template(&req).await
    }

    pub async fn get_form_templates(
        &self,
        school_id: Uuid,
    ) -> ApiResult<Vec<EmployeeFormTemplate>> {
        self.employee_form_template_dao
            .get_templates_by_school(school_id)
            .await
    }

    pub async fn update_form_template(
        &self,
        req: UpdateEmployeeFormTemplateRequest,
    ) -> ApiResult<EmployeeFormTemplate> {
        self.employee_form_template_dao.update_template(&req).await
    }

    pub async fn delete_form_template(&self, form_id: Uuid, school_id: Uuid) -> ApiResult<()> {
        let pdf_key = self
            .employee_form_template_dao
            .get_template_by_id(form_id, school_id)
            .await?
            .and_then(|template| template.pdf_storage_key);
        self.employee_form_template_dao
            .delete_template(form_id, school_id)
            .await?;
        if let Some(key) = pdf_key {
            if let Err(error) = self.upload_service.delete_document_object(&key).await {
                tracing::warn!("Unable to remove deleted employee template PDF: {}", error);
            }
        }
        Ok(())
    }

    pub async fn employee_template_pdf_upload_intent(
        &self,
        id: Uuid,
        school_id: Uuid,
        data: &UploadIntentRequest,
    ) -> ApiResult<UploadIntentResponse> {
        if data.content_type != "application/pdf"
            || data.file_size_bytes <= 0
            || data.file_size_bytes > crate::services::upload_service::DOCUMENT_MAX_SIZE_BYTES
        {
            return Err(AppError::Validation(
                "Upload a PDF template no larger than 10 MB".into(),
            ));
        }
        self.employee_form_template_dao
            .get_template_by_id(id, school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee form template not found".into()))?;
        let key = format!(
            "private/schools/{}/employee-form-templates/{}/{}.pdf",
            school_id,
            id,
            Uuid::new_v4()
        );
        Ok(UploadIntentResponse {
            storage_key: key.clone(),
            upload_url: self
                .upload_service
                .create_document_upload_url(&key, &data.content_type, data.file_size_bytes)
                .await?,
            expires_in_seconds: 300,
        })
    }

    pub async fn complete_employee_template_pdf_upload(
        &self,
        id: Uuid,
        school_id: Uuid,
        data: &CompleteUploadRequest,
    ) -> ApiResult<EmployeeFormTemplate> {
        if data.content_type != "application/pdf"
            || !data.storage_key.starts_with(&format!(
                "private/schools/{}/employee-form-templates/{}/",
                school_id, id
            ))
        {
            return Err(AppError::Validation(
                "Invalid employee form template PDF upload".into(),
            ));
        }
        self.upload_service
            .verify_document_object(&data.storage_key, &data.content_type, data.file_size_bytes)
            .await?;
        let previous = self
            .employee_form_template_dao
            .get_template_by_id(id, school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee form template not found".into()))?;
        let updated = self
            .employee_form_template_dao
            .set_pdf(
                id,
                school_id,
                &data.storage_key,
                &data.file_name,
                &data.content_type,
                data.file_size_bytes,
            )
            .await?;
        if let Some(old_key) = previous
            .pdf_storage_key
            .filter(|key| key != &data.storage_key)
        {
            if let Err(error) = self.upload_service.delete_document_object(&old_key).await {
                tracing::warn!("Unable to remove replaced employee template PDF: {}", error);
            }
        }
        Ok(updated)
    }

    pub async fn employee_template_pdf_access_url(
        &self,
        id: Uuid,
        school_id: Uuid,
        download: bool,
    ) -> ApiResult<FileAccessResponse> {
        let template = self
            .employee_form_template_dao
            .get_template_by_id(id, school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee form template not found".into()))?;
        let key = template
            .pdf_storage_key
            .ok_or_else(|| AppError::NotFound("No PDF template is attached".into()))?;
        Ok(FileAccessResponse {
            url: self
                .upload_service
                .create_document_access_url(&key, download)
                .await?,
            expires_in_seconds: 300,
        })
    }

    pub async fn remove_employee_template_pdf(&self, id: Uuid, school_id: Uuid) -> ApiResult<()> {
        if let Some(key) = self
            .employee_form_template_dao
            .clear_pdf(id, school_id)
            .await?
        {
            self.upload_service.delete_document_object(&key).await?;
        }
        Ok(())
    }

    // ─── Employee Form Assignments ──────────────────────────────────────────────

    pub async fn assign_form(
        &self,
        req: AssignEmployeeFormRequest,
        assigned_by: Uuid,
    ) -> ApiResult<EmployeeFormAssignment> {
        // Get employee to find user_id and email
        let employee = self
            .employee_dao
            .get_employee_by_id(req.employee_id, req.school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee not found".to_string()))?;

        // Get template to send notification with form name / due date
        let template = self
            .employee_form_template_dao
            .get_template_by_id(req.employee_form_template_id, req.school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee form template not found".to_string()))?;

        let is_required = req.is_required.unwrap_or(false);

        let assignment = self
            .employee_form_assignment_dao
            .create_assignment(
                req.employee_id,
                employee.user_id,
                req.school_id,
                req.employee_form_template_id,
                assigned_by,
                is_required,
            )
            .await?;

        // Send notification email (non-fatal)
        let due_date_str = template
            .due_date
            .map(|d| d.format("%B %d, %Y").to_string())
            .unwrap_or_default();
        let employee_name = format!("{} {}", employee.first_name, employee.last_name);

        let email_svc = self.email_service.clone();
        let email = employee.email.clone();
        let form_name = template.form_name.clone();
        let dashboard_url = Self::employee_dashboard_url();

        tokio::spawn(async move {
            if let Err(e) = email_svc
                .send_employee_form_assigned_email(
                    &email,
                    &employee_name,
                    &form_name,
                    &due_date_str,
                    &dashboard_url,
                )
                .await
            {
                tracing::warn!("Employee form assigned email failed: {}", e);
            }
        });

        Ok(assignment)
    }

    pub async fn assign_form_to_all_employees(
        &self,
        req: AssignEmployeeFormToSchoolRequest,
        assigned_by: Uuid,
    ) -> ApiResult<AssignEmployeeFormToSchoolResponse> {
        let template = self
            .employee_form_template_dao
            .get_template_by_id(req.employee_form_template_id, req.school_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee form template not found".to_string()))?;

        if template.status.as_deref() == Some("inactive") || template.is_active == Some(false) {
            return Err(AppError::Validation(
                "Cannot assign an inactive employee form template".to_string(),
            ));
        }

        let (total_active_employees, employees_already_assigned, newly_assigned) = self
            .employee_form_assignment_dao
            .assign_template_to_school_employees(
                req.school_id,
                req.employee_form_template_id,
                assigned_by,
                req.is_required.unwrap_or(false),
            )
            .await?;

        Ok(AssignEmployeeFormToSchoolResponse {
            school_id: req.school_id,
            employee_form_template_id: req.employee_form_template_id,
            total_active_employees,
            employees_already_assigned,
            newly_assigned,
        })
    }

    pub async fn get_assignments_by_employee(
        &self,
        employee_id: Uuid,
    ) -> ApiResult<Vec<EmployeeFormAssignmentWithTemplate>> {
        self.employee_form_assignment_dao
            .get_assignments_by_employee(employee_id)
            .await
    }

    pub async fn get_assignments_by_school(
        &self,
        school_id: Uuid,
    ) -> ApiResult<Vec<EmployeeFormAssignmentWithTemplate>> {
        self.employee_form_assignment_dao
            .get_assignments_by_school(school_id)
            .await
    }

    pub async fn get_review_queue(
        &self,
        query: &FormReviewQueueQuery,
    ) -> ApiResult<Vec<EmployeeFormReviewQueueItem>> {
        let mut items = self
            .employee_form_assignment_dao
            .get_review_queue(query.school_id)
            .await?;
        if let Some(form_template_id) = query.form_template_id {
            items.retain(|item| item.form_template_id == form_template_id);
        }
        if let Some(search) = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let needle = search.to_lowercase();
            items.retain(|item| {
                format!(
                    "{} {} {}",
                    item.employee_first_name, item.employee_last_name, item.employee_email
                )
                .to_lowercase()
                .contains(&needle)
            });
        }
        let ascending = query
            .sort_direction
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("asc"))
            .unwrap_or(false);
        if query
            .sort_by
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("name"))
            .unwrap_or(false)
        {
            items.sort_by_key(|item| {
                format!(
                    "{} {}",
                    item.employee_first_name.to_lowercase(),
                    item.employee_last_name.to_lowercase()
                )
            });
        } else {
            items.sort_by_key(|item| item.submitted_at);
        }
        if !ascending {
            items.reverse();
        }
        Ok(items)
    }

    pub async fn review_assignment(
        &self,
        req: ReviewEmployeeFormRequest,
        reviewer_id: Uuid,
    ) -> ApiResult<EmployeeFormAssignment> {
        let assignment = self
            .employee_form_assignment_dao
            .review_assignment(
                req.assignment_id,
                req.school_id,
                &req.status,
                reviewer_id,
                req.notes.as_deref(),
            )
            .await?;

        // Send approval/rejection email (non-fatal)
        let notes = req.notes.unwrap_or_default();
        let status = req.status.clone();
        let employee = self
            .employee_dao
            .get_employee_by_id(assignment.employee_id, req.school_id)
            .await;
        let template = self
            .employee_form_template_dao
            .get_template_by_id(assignment.employee_form_template_id, req.school_id)
            .await;

        if let (Ok(Some(emp)), Ok(Some(tpl))) = (employee, template) {
            let email_svc = self.email_service.clone();
            let email = emp.email;
            let name = format!("{} {}", emp.first_name, emp.last_name);
            let form_name = tpl.form_name;

            tokio::spawn(async move {
                let result = if status == "approved" {
                    email_svc
                        .send_employee_form_approved_email(&email, &name, &form_name, &notes)
                        .await
                } else {
                    email_svc
                        .send_employee_form_rejected_email(&email, &name, &form_name, &notes)
                        .await
                };
                if let Err(e) = result {
                    tracing::warn!("Employee form review email failed: {}", e);
                }
            });
        }

        Ok(assignment)
    }

    pub async fn delete_assignment(&self, assignment_id: Uuid, school_id: Uuid) -> ApiResult<()> {
        self.employee_form_assignment_dao
            .delete_assignment(assignment_id, school_id)
            .await
    }

    // ─── Form Submissions (Fillout webhook) ─────────────────────────────────────

    pub async fn handle_form_webhook(
        &self,
        assignment_id: Uuid,
        fillout_submission_id: &str,
        form_data: Option<&serde_json::Value>,
        metadata: Option<&serde_json::Value>,
        edit_link: Option<&str>,
        pdf_link: Option<&str>,
        submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> ApiResult<EmployeeFormSubmission> {
        let (school_id, employee_id, template_id) = self
            .employee_form_assignment_dao
            .get_assignment_details(assignment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employee form assignment not found".to_string()))?;

        let submission = self
            .employee_form_submission_dao
            .upsert_submission(
                school_id,
                employee_id,
                assignment_id,
                template_id,
                fillout_submission_id,
                form_data,
                metadata,
                edit_link,
                pdf_link,
                submitted_at,
            )
            .await?;

        let _ = self
            .employee_form_assignment_dao
            .update_assignment_status(assignment_id, "in_progress", edit_link, pdf_link)
            .await;

        Ok(submission)
    }

    pub async fn get_submission_by_assignment(
        &self,
        assignment_id: Uuid,
    ) -> ApiResult<Option<EmployeeFormSubmission>> {
        self.employee_form_submission_dao
            .get_submission_by_assignment(assignment_id)
            .await
    }

    pub async fn get_submissions_by_employee(
        &self,
        employee_id: Uuid,
    ) -> ApiResult<Vec<EmployeeFormSubmission>> {
        self.employee_form_submission_dao
            .get_submissions_by_employee(employee_id)
            .await
    }

    // ─── Bulk Reminders ─────────────────────────────────────────────────────────

    pub async fn send_bulk_reminders(
        &self,
        req: BulkEmployeeFormReminderRequest,
    ) -> ApiResult<BulkEmployeeReminderResponse> {
        let dashboard_url = Self::employee_dashboard_url();
        let mut total_sent: i32 = 0;
        let mut total_failed: i32 = 0;
        let mut failed_emails: Vec<String> = Vec::new();

        for reminder in &req.reminders {
            let result = self
                .email_service
                .send_employee_form_reminder_email(
                    &reminder.employee_email,
                    &reminder.employee_name,
                    &reminder.form_name,
                    &reminder.due_date,
                    &dashboard_url,
                )
                .await;

            match result {
                Ok(_) => total_sent += 1,
                Err(e) => {
                    tracing::warn!(
                        "Reminder email failed for {}: {}",
                        reminder.employee_email,
                        e
                    );
                    total_failed += 1;
                    failed_emails.push(reminder.employee_email.clone());
                }
            }
        }

        Ok(BulkEmployeeReminderResponse {
            total_sent,
            total_failed,
            failed_emails,
            message: format!("Sent {} reminder(s), {} failed.", total_sent, total_failed),
        })
    }
}
