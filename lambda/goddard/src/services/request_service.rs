use std::sync::Arc;
use uuid::Uuid;
use crate::dao::{request_dao::RequestDao, SchoolDao};
use crate::models::school::SchoolFeature;
use crate::middleware::auth::AuthContext;
use crate::models::schema::UserRole;
use crate::models::requests::{
    CreateRequestBody, UpdateRequestStatusBody, UpdateExpectedCompletionDateBody, UpdateRequestBody, PayRequestBody, CreateExpenseBody,
    ListRequestsParams, ListExpensesParams,
    RequestsListResponse, RequestStatusCounts, ExpensesListResponse, Request,
};
use crate::services::upload_service::UploadService;
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct RequestService {
    dao: RequestDao,
    upload_service: Arc<UploadService>,
    schools: SchoolDao,
}

impl RequestService {
    pub fn new(dao: RequestDao, upload_service: Arc<UploadService>, schools: SchoolDao) -> Self {
        Self { dao, upload_service, schools }
    }

    async fn ensure_enabled(&self, school_id: Uuid) -> Result<(), AppError> {
        self.schools.ensure_feature_enabled(school_id, SchoolFeature::ExpenseManagement).await
    }

    // ── Requests ──────────────────────────────────────────────────────────────

    pub async fn list_requests(
        &self,
        auth: &AuthContext,
        mut params: ListRequestsParams,
    ) -> Result<RequestsListResponse, AppError> {
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(20).min(100);

        match auth.role {
            UserRole::SuperAdmin => {
                // Optional school filter from params; sees everything otherwise
            }
            UserRole::Admin => {
                params.school_id = Some(auth.school_id);
            }
            _ => {
                // Teacher / employee / parent: own school, own requests only
                params.school_id = Some(auth.school_id);
                params.user_id = Some(auth.user_id);
            }
        }

        // A global SuperAdmin view is not associated with one school. Individual
        // records are still filtered by the feature at their school in the UI;
        // school-scoped requests are always checked here.
        if let Some(school_id) = params.school_id { self.ensure_enabled(school_id).await?; }

        let (data, total, pending, in_progress, completed) = self.dao
            .list_requests(
                params.school_id,
                params.user_id,
                params.status.as_deref(),
                params.requester_role.as_deref(),
                page,
                limit,
            )
            .await?;

        Ok(RequestsListResponse {
            data,
            total,
            page,
            limit,
            counts: RequestStatusCounts { pending, in_progress, completed },
        })
    }

    pub async fn create_request(
        &self,
        auth: &AuthContext,
        mut body: CreateRequestBody,
    ) -> Result<Request, AppError> {
        self.ensure_enabled(body.school_id).await?;
        match auth.role {
            UserRole::SuperAdmin => {}
            _ => {
                if body.school_id != auth.school_id {
                    return Err(AppError::Authorization(
                        "Cannot create request for a different school".to_string(),
                    ));
                }
            }
        }
        if body.item.trim().is_empty() {
            return Err(AppError::Validation("Item name cannot be empty".to_string()));
        }
        if body.quantity < 1 {
            return Err(AppError::Validation("Quantity must be at least 1".to_string()));
        }
        self.dao.validate_request_settings(
            body.school_id,
            body.category.as_deref(),
            body.location.as_deref(),
            body.scope == "school",
        ).await?;

        // If frontend sent image as base64, decode and upload to S3
        if let (Some(b64), Some(name), Some(ct)) = (
            body.image_base64.take(),
            body.image_name.take(),
            body.image_content_type.take(),
        ) {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let bytes = STANDARD.decode(&b64)
                .map_err(|e| AppError::Validation(format!("Invalid base64 image: {}", e)))?;
            let resp = self.upload_service.upload_image(&name, &ct, bytes).await?;
            body.product_image = Some(resp.s3_url);
        }

        self.dao.create_request(&body).await
    }

    pub async fn update_request_status(
        &self,
        auth: &AuthContext,
        id: Uuid,
        body: UpdateRequestStatusBody,
    ) -> Result<Request, AppError> {
        let valid = ["Pending", "In Progress", "Completed"];
        if !valid.contains(&body.status.as_str()) {
            return Err(AppError::Validation(format!(
                "Invalid status '{}'. Must be one of: Pending, In Progress, Completed",
                body.status
            )));
        }

        let req = self.dao.get_request_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;
        self.ensure_enabled(req.school_id).await?;

        match auth.role {
            UserRole::SuperAdmin => {}
            UserRole::Admin => {
                if req.school_id != auth.school_id {
                    return Err(AppError::Authorization(
                        "Cannot update a request from a different school".to_string(),
                    ));
                }
            }
            _ => {
                return Err(AppError::Authorization(
                    "Only admin or superadmin can update request status".to_string(),
                ));
            }
        }

        let expected_completion_date = if body.status == "In Progress" {
            Some(body.expected_completion_date.ok_or_else(|| AppError::Validation(
                "expectedCompletionDate is required when moving a request to In Progress".to_string()
            ))?)
        } else {
            None
        };

        self.dao.update_request_status(id, &body.status, expected_completion_date).await
    }

    pub async fn update_expected_completion_date(
        &self,
        auth: &AuthContext,
        id: Uuid,
        body: UpdateExpectedCompletionDateBody,
    ) -> Result<Request, AppError> {
        let req = self.authorize_admin_request(auth, id).await?;
        self.ensure_enabled(req.school_id).await?;
        if req.status != "In Progress" {
            return Err(AppError::Validation("Expected completion date can only be updated for an In Progress request".to_string()));
        }
        self.dao.update_expected_completion_date(id, body.expected_completion_date).await
    }

    pub async fn update_request(
        &self,
        auth: &AuthContext,
        id: Uuid,
        mut body: UpdateRequestBody,
    ) -> Result<Request, AppError> {
        let existing = self.authorize_request_editor(auth, id).await?;
        self.ensure_enabled(existing.school_id).await?;
        if let Some(item) = &body.item {
            if item.trim().is_empty() {
                return Err(AppError::Validation("Item name cannot be empty".to_string()));
            }
        }
        if let Some(quantity) = body.quantity {
            if quantity < 1 {
                return Err(AppError::Validation("Quantity must be at least 1".to_string()));
            }
        }
        let effective_scope = body.scope.as_deref()
            .or(existing.scope.as_deref())
            .unwrap_or("school");
        self.dao.validate_request_settings(
            existing.school_id,
            body.category.as_deref().or(existing.category.as_deref()),
            body.location.as_deref().or(existing.location.as_deref()),
            effective_scope == "school",
        ).await?;
        if let (Some(base64), Some(name), Some(content_type)) = (
            body.image_base64.take(),
            body.image_name.take(),
            body.image_content_type.take(),
        ) {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let bytes = STANDARD.decode(&base64)
                .map_err(|error| AppError::Validation(format!("Invalid product image: {}", error)))?;
            let uploaded = self.upload_service.upload_image(&name, &content_type, bytes).await?;
            body.product_image = Some(uploaded.s3_url);
        }
        self.dao.update_request(id, &body).await
    }

    pub async fn pay_request(&self, id: Uuid, mut body: PayRequestBody) -> Result<Request, AppError> {
        let request = self.dao.get_request_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;
        self.ensure_enabled(request.school_id).await?;

        if body.amount_spent <= 0.0 {
            return Err(AppError::Validation("Amount must be greater than 0".to_string()));
        }

        let mut bill_image_url: Option<String> = None;
        if let (Some(b64), Some(name), Some(ct)) = (
            body.bill_image_base64.take(),
            body.bill_image_name.take(),
            body.bill_image_content_type.take(),
        ) {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let bytes = STANDARD.decode(&b64)
                .map_err(|e| AppError::Validation(format!("Invalid base64 bill image: {}", e)))?;
            let resp = self.upload_service.upload_image(&name, &ct, bytes).await?;
            bill_image_url = Some(resp.s3_url);
        }

        self.dao.pay_request(
            id,
            body.amount_spent,
            &body.payment_method,
            body.purchase_date,
            body.payment_notes.as_deref(),
            bill_image_url.as_deref(),
        ).await
    }

    pub async fn delete_request(&self, auth: &AuthContext, id: Uuid) -> Result<(), AppError> {
        let request = self.authorize_request_editor(auth, id).await?;
        self.ensure_enabled(request.school_id).await?;
        self.dao.delete_request(id).await
    }

    async fn authorize_request_editor(&self, auth: &AuthContext, id: Uuid) -> Result<Request, AppError> {
        let req = self.dao.get_request_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;
        match auth.role {
            UserRole::SuperAdmin => Ok(req),
            UserRole::Admin if req.school_id == auth.school_id => Ok(req),
            UserRole::Admin => Err(AppError::Authorization("Cannot modify a request from a different school".to_string())),
            UserRole::Teacher if req.school_id == auth.school_id && req.requester_id == Some(auth.user_id) => {
                if req.status != "Pending" {
                    return Err(AppError::Validation("Employees can only edit or delete Pending requests".to_string()));
                }
                Ok(req)
            }
            UserRole::Teacher => Err(AppError::Authorization("Employees can only modify their own requests".to_string())),
            _ => Err(AppError::Authorization("Only authorized request owners can modify requests".to_string())),
        }
    }

    async fn authorize_admin_request(&self, auth: &AuthContext, id: Uuid) -> Result<Request, AppError> {
        let req = self.dao.get_request_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;
        match auth.role {
            UserRole::SuperAdmin => Ok(req),
            UserRole::Admin if req.school_id == auth.school_id => Ok(req),
            UserRole::Admin => Err(AppError::Authorization("Cannot update a request from a different school".to_string())),
            _ => Err(AppError::Authorization("Only admin or superadmin can update requests".to_string())),
        }
    }

    // ── Expenses ──────────────────────────────────────────────────────────────

    pub async fn list_expenses(&self, params: ListExpensesParams) -> Result<ExpensesListResponse, AppError> {
        let school_id = params.school_id.ok_or_else(|| AppError::Validation("school_id is required".into()))?;
        self.ensure_enabled(school_id).await?;
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(20).min(100);
        let include_summary = params.include.as_deref() == Some("summary");

        let (data, total) = self.dao
            .list_expenses(params.school_id, params.search.as_deref(), page, limit)
            .await?;

        let summary = if include_summary {
            Some(self.dao.get_expense_summary(params.school_id).await?)
        } else {
            None
        };

        Ok(ExpensesListResponse { data, total, page, limit, summary })
    }

    pub async fn create_manual_expense(&self, body: CreateExpenseBody) -> Result<Request, AppError> {
        self.ensure_enabled(body.school_id).await?;
        if body.item.trim().is_empty() {
            return Err(AppError::Validation("Item name cannot be empty".to_string()));
        }
        if body.amount_spent <= 0.0 {
            return Err(AppError::Validation("Amount must be greater than 0".to_string()));
        }
        self.dao.create_manual_expense(&body).await
    }
}
