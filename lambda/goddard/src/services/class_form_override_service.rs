use uuid::Uuid;
use crate::dao::class_form_override_dao::ClassFormOverrideDao;
use crate::models::class_form_override::{ClassFormOverride, CreateClassFormOverrideRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct ClassFormOverrideService {
    dao: ClassFormOverrideDao,
}

impl ClassFormOverrideService {
    pub fn new(dao: ClassFormOverrideDao) -> Self {
        Self { dao }
    }

    pub async fn create_class_form_override(&self, request: CreateClassFormOverrideRequest) -> Result<ClassFormOverride, AppError> {
        // Basic validation
        if request.school_id.to_string().is_empty() || request.classroom_id.to_string().is_empty() || request.form_template_id.to_string().is_empty() {
            return Err(AppError::Validation("School ID, classroom ID, and form template ID are required".to_string()));
        }

        self.dao.create_class_form_override(&request).await
    }

    pub async fn delete_class_form_override(&self, override_id: Uuid) -> Result<ClassFormOverride, AppError> {
        self.dao.delete_class_form_override(&override_id).await
    }
}