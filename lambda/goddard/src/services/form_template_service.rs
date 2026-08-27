use std::sync::Arc;
use uuid::Uuid;

use crate::dao::form_template_dao::FormTemplateDao;
use crate::dao::school_dao::SchoolDao;
use crate::error::{error_types::AppError, ApiResult};
use crate::models::form_template::{FormTemplate, CreateFormTemplateRequest, UpdateFormTemplateRequest};
use crate::models::document_request::{UploadIntentRequest, UploadIntentResponse, CompleteUploadRequest, FileAccessResponse};
use crate::models::notification::{notification_type, CreateNotification};
use crate::services::{NotificationService, UploadService};

#[derive(Clone)]
pub struct FormTemplateService {
    dao: FormTemplateDao,
    school_dao: SchoolDao,
    notification_service: Arc<NotificationService>,
    upload_service: Arc<UploadService>,
}

impl FormTemplateService {
    pub fn new(
        dao: FormTemplateDao,
        school_dao: SchoolDao,
        notification_service: Arc<NotificationService>,
        upload_service: Arc<UploadService>,
    ) -> Self {
        Self {
            dao,
            school_dao,
            notification_service,
            upload_service,
        }
    }

    pub async fn create_form_template(&self, request: CreateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        if request.form_name.trim().is_empty() {
            return Err(AppError::Validation("Form name cannot be empty".to_string()));
        }

        // Validate due_date is not in the past
        if let Some(due_date) = request.due_date {
            let today = chrono::Local::now().date_naive();
            if due_date < today {
                return Err(AppError::Validation(
                    "due_date must be greater than or equal to current date".to_string()
                ));
            }
        }

        // Validate status if provided
        if let Some(ref status) = request.status {
            let valid_statuses = vec!["active", "inactive", "draft", "archived", "school_default", "available"];
            if !valid_statuses.contains(&status.as_str()) {
                return Err(AppError::Validation(format!(
                    "Invalid status '{}'. Must be one of: {}",
                    status,
                    valid_statuses.join(", ")
                )));
            }
        }

        let template = self.dao.create_form_template(&request).await?;

        let school_name = self
            .school_dao
            .get_school_name(&template.school_id)
            .await
            .unwrap_or_default();
        let body = if school_name.is_empty() {
            format!("Form template \"{}\" has been added.", template.form_name)
        } else {
            format!(
                "Form template \"{}\" has been added to {}.",
                template.form_name, school_name
            )
        };

        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id: template.school_id,
                notification_type: notification_type::FORM_TEMPLATE_ADDED.to_string(),
                title: "New Form Added".to_string(),
                body,
                related_entity_id: Some(template.id),
                related_entity_type: Some("form_template".to_string()),
                action_url: Some("/admin/forms".to_string()),
            },
            None,
        ).await;

        Ok(template)
    }

    pub async fn get_form_templates_by_school(&self, school_id: Uuid) -> Result<Vec<FormTemplate>, AppError> {
        self.dao.get_form_templates_by_school(&school_id).await
    }

    pub async fn update_form_template(&self, request: UpdateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        if request.form_name.trim().is_empty() {
            return Err(AppError::Validation("Form name cannot be empty".to_string()));
        }

        // Validate due_date is not in the past
        if let Some(due_date) = request.due_date {
            let today = chrono::Local::now().date_naive();
            if due_date < today {
                return Err(AppError::Validation(
                    "due_date must be greater than or equal to current date".to_string()
                ));
            }
        }

        self.dao.update_form_template(&request).await
    }

    pub async fn delete_form_template(&self, form_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        // Fetch the name BEFORE the delete so the notification body can show it.
        let form_name = self
            .dao
            .get_form_template_name(&form_id)
            .await
            .ok()
            .flatten();
        let pdf_key = self.dao.get_form_template_by_id(form_id, school_id).await?.and_then(|template| template.pdf_storage_key);

        self.dao.delete_form_template(&form_id, &school_id).await?;
        if let Some(key) = pdf_key { if let Err(error) = self.upload_service.delete_document_object(&key).await { tracing::warn!("Unable to remove deleted form template PDF: {}", error); } }

        let body = match form_name {
            Some(name) if !name.is_empty() => format!("Form template \"{}\" has been deleted.", name),
            _ => "A form template has been deleted.".to_string(),
        };

        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id,
                notification_type: notification_type::FORM_TEMPLATE_DELETED.to_string(),
                title: "Form Template Deleted".to_string(),
                body,
                related_entity_id: Some(form_id),
                related_entity_type: Some("form_template".to_string()),
                action_url: Some("/admin/forms".to_string()),
            },
            None,
        ).await;

        Ok(())
    }

    fn validate_template_pdf(data: &UploadIntentRequest) -> ApiResult<()> {
        if data.content_type != "application/pdf" {
            return Err(AppError::Validation("Form template uploads must be PDF files".into()));
        }
        if data.file_size_bytes <= 0 || data.file_size_bytes > crate::services::upload_service::DOCUMENT_MAX_SIZE_BYTES {
            return Err(AppError::Validation("PDF template size must be between 1 byte and 10 MB".into()));
        }
        Ok(())
    }

    pub async fn pdf_upload_intent(&self, id: Uuid, school_id: Uuid, data: &UploadIntentRequest) -> ApiResult<UploadIntentResponse> {
        Self::validate_template_pdf(data)?;
        self.dao.get_form_template_by_id(id, school_id).await?.ok_or_else(|| AppError::NotFound("Form template not found".into()))?;
        let key = format!("private/schools/{}/form-templates/{}/{}.pdf", school_id, id, Uuid::new_v4());
        let upload_url = self.upload_service.create_document_upload_url(&key, &data.content_type, data.file_size_bytes).await?;
        Ok(UploadIntentResponse { storage_key: key, upload_url, expires_in_seconds: 300 })
    }

    pub async fn complete_pdf_upload(&self, id: Uuid, school_id: Uuid, data: &CompleteUploadRequest) -> ApiResult<FormTemplate> {
        if data.content_type != "application/pdf" || !data.storage_key.starts_with(&format!("private/schools/{}/form-templates/{}/", school_id, id)) {
            return Err(AppError::Validation("Invalid form template PDF upload".into()));
        }
        self.upload_service.verify_document_object(&data.storage_key, &data.content_type, data.file_size_bytes).await?;
        let previous = self.dao.get_form_template_by_id(id, school_id).await?.ok_or_else(|| AppError::NotFound("Form template not found".into()))?;
        let updated = self.dao.set_pdf(id, school_id, &data.storage_key, &data.file_name, &data.content_type, data.file_size_bytes).await?;
        if let Some(old_key) = previous.pdf_storage_key.filter(|key| key != &data.storage_key) {
            if let Err(error) = self.upload_service.delete_document_object(&old_key).await { tracing::warn!("Unable to remove replaced form template PDF: {}", error); }
        }
        Ok(updated)
    }

    pub async fn pdf_access_url(&self, id: Uuid, school_id: Uuid, download: bool) -> ApiResult<FileAccessResponse> {
        let template = self.dao.get_form_template_by_id(id, school_id).await?.ok_or_else(|| AppError::NotFound("Form template not found".into()))?;
        let key = template.pdf_storage_key.ok_or_else(|| AppError::NotFound("No PDF template is attached".into()))?;
        Ok(FileAccessResponse { url: self.upload_service.create_document_access_url(&key, download).await?, expires_in_seconds: 300 })
    }

    pub async fn remove_pdf(&self, id: Uuid, school_id: Uuid) -> ApiResult<()> {
        if let Some(key) = self.dao.clear_pdf(id, school_id).await? {
            self.upload_service.delete_document_object(&key).await?;
        }
        Ok(())
    }
}
