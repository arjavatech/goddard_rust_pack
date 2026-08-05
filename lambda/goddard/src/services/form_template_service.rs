use std::sync::Arc;
use uuid::Uuid;

use crate::dao::form_template_dao::FormTemplateDao;
use crate::dao::school_dao::SchoolDao;
use crate::error::error_types::AppError;
use crate::models::form_template::{FormTemplate, CreateFormTemplateRequest, UpdateFormTemplateRequest};
use crate::models::notification::{notification_type, CreateNotification};
use crate::services::NotificationService;

#[derive(Clone)]
pub struct FormTemplateService {
    dao: FormTemplateDao,
    school_dao: SchoolDao,
    notification_service: Arc<NotificationService>,
}

impl FormTemplateService {
    pub fn new(
        dao: FormTemplateDao,
        school_dao: SchoolDao,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            dao,
            school_dao,
            notification_service,
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
                action_url: None,
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

        self.dao.delete_form_template(&form_id, &school_id).await?;

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
                action_url: None,
            },
            None,
        ).await;

        Ok(())
    }
}