use uuid::Uuid;
use crate::dao::form_template_dao::FormTemplateDao;
use crate::models::form_template::{FormTemplate, CreateFormTemplateRequest, UpdateFormTemplateRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct FormTemplateService {
    dao: FormTemplateDao,
}

impl FormTemplateService {
    pub fn new(dao: FormTemplateDao) -> Self {
        Self { dao }
    }

    pub async fn create_form_template(&self, request: CreateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        if request.form_name.trim().is_empty() {
            return Err(AppError::Validation("Form name cannot be empty".to_string()));
        }

        self.dao.create_form_template(&request).await
    }

    pub async fn get_form_templates_by_school(&self, school_id: Uuid) -> Result<Vec<FormTemplate>, AppError> {
        self.dao.get_form_templates_by_school(&school_id).await
    }

    pub async fn update_form_template(&self, request: UpdateFormTemplateRequest) -> Result<FormTemplate, AppError> {
        if request.form_name.trim().is_empty() {
            return Err(AppError::Validation("Form name cannot be empty".to_string()));
        }

        self.dao.update_form_template(&request).await
    }

    pub async fn delete_form_template(&self, form_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        self.dao.delete_form_template(&form_id, &school_id).await
    }
}