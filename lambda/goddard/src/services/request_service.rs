use std::sync::Arc;
use uuid::Uuid;
use crate::dao::request_dao::RequestDao;
use crate::middleware::auth::AuthContext;
use crate::models::schema::UserRole;
use crate::models::requests::{
    CreateRequestBody, UpdateRequestStatusBody, PayRequestBody, CreateExpenseBody,
    ListRequestsParams, ListExpensesParams,
    RequestsListResponse, RequestStatusCounts, ExpensesListResponse, Request,
};
use crate::services::upload_service::UploadService;
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct RequestService {
    dao: RequestDao,
    upload_service: Arc<UploadService>,
}

impl RequestService {
    pub fn new(dao: RequestDao, upload_service: Arc<UploadService>) -> Self {
        Self { dao, upload_service }
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

        self.dao.update_request_status(id, &body.status).await
    }

    pub async fn pay_request(&self, id: Uuid, body: PayRequestBody) -> Result<Request, AppError> {
        self.dao.get_request_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;

        if body.amount_spent <= 0.0 {
            return Err(AppError::Validation("Amount must be greater than 0".to_string()));
        }

        self.dao.pay_request(id, body.amount_spent, &body.payment_method, body.purchase_date, body.payment_notes.as_deref()).await
    }

    pub async fn delete_request(&self, auth: &AuthContext, id: Uuid) -> Result<(), AppError> {
        let req = self.dao.get_request_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;

        match auth.role {
            UserRole::SuperAdmin => {}
            UserRole::Admin => {
                if req.school_id != auth.school_id {
                    return Err(AppError::Authorization(
                        "Cannot delete a request from a different school".to_string(),
                    ));
                }
            }
            _ => {
                return Err(AppError::Authorization(
                    "Only admin or superadmin can delete requests".to_string(),
                ));
            }
        }

        self.dao.delete_request(id).await
    }

    // ── Expenses ──────────────────────────────────────────────────────────────

    pub async fn list_expenses(&self, params: ListExpensesParams) -> Result<ExpensesListResponse, AppError> {
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
        if body.item.trim().is_empty() {
            return Err(AppError::Validation("Item name cannot be empty".to_string()));
        }
        if body.amount_spent <= 0.0 {
            return Err(AppError::Validation("Amount must be greater than 0".to_string()));
        }
        self.dao.create_manual_expense(&body).await
    }
}
