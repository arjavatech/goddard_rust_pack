use crate::{
    dao::SchoolDao,
    error::{AppError, ApiResult},
    models::school::{
        CreateSchoolRequest, UpdateSchoolRequest,
        SchoolResponse, SchoolListItem, DeleteSchoolResponse,
    },
};
use uuid::Uuid;

pub struct SchoolService {
    dao: SchoolDao,
}

impl SchoolService {
    pub fn new(dao: SchoolDao) -> Self {
        Self { dao }
    }

    pub async fn create_school(&self, request: CreateSchoolRequest) -> ApiResult<SchoolResponse> {
        // Validate input
        if request.name.trim().is_empty() {
            return Err(AppError::Validation("School name cannot be empty".to_string()));
        }

        if request.subdomain.trim().is_empty() {
            return Err(AppError::Validation("Subdomain cannot be empty".to_string()));
        }

        // Validate subdomain format (alphanumeric and hyphens only)
        if !request.subdomain.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(AppError::Validation("Subdomain can only contain letters, numbers, and hyphens".to_string()));
        }

        // Check if subdomain already exists
        if self.dao.check_subdomain_exists(&request.subdomain, None).await? {
            return Err(AppError::Conflict("Subdomain already exists".to_string()));
        }

        let school = self.dao.create_school(&request).await?;
        Ok(SchoolResponse::from(school))
    }

    pub async fn get_all_schools(&self) -> ApiResult<Vec<SchoolListItem>> {
        let schools = self.dao.get_all_schools().await?;
        let school_list = schools.into_iter().map(SchoolListItem::from).collect();
        Ok(school_list)
    }

    pub async fn get_school_by_id(&self, school_id: &Uuid) -> ApiResult<SchoolResponse> {
        let school = self.dao.get_school_by_id(school_id).await?
            .ok_or_else(|| AppError::NotFound("School not found".to_string()))?;
        Ok(SchoolResponse::from(school))
    }

    pub async fn update_school(&self, request: UpdateSchoolRequest) -> ApiResult<SchoolResponse> {
        // Validate input
        if request.name.trim().is_empty() {
            return Err(AppError::Validation("School name cannot be empty".to_string()));
        }

        if request.subdomain.trim().is_empty() {
            return Err(AppError::Validation("Subdomain cannot be empty".to_string()));
        }

        // Validate subdomain format
        if !request.subdomain.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(AppError::Validation("Subdomain can only contain letters, numbers, and hyphens".to_string()));
        }

        // Check if subdomain already exists (excluding current school)
        if self.dao.check_subdomain_exists(&request.subdomain, Some(&request.id)).await? {
            return Err(AppError::Conflict("Subdomain already exists".to_string()));
        }

        // Check if school exists
        if self.dao.get_school_by_id(&request.id).await?.is_none() {
            return Err(AppError::NotFound("School not found".to_string()));
        }

        let school = self.dao.update_school(&request).await?;
        Ok(SchoolResponse::from(school))
    }

    pub async fn delete_school(&self, school_id: &Uuid) -> ApiResult<DeleteSchoolResponse> {
        // Check if school exists
        if self.dao.get_school_by_id(school_id).await?.is_none() {
            return Err(AppError::NotFound("School not found".to_string()));
        }

        self.dao.delete_school(school_id).await?;

        Ok(DeleteSchoolResponse {
            message: "School successfully deleted".to_string(),
            school_id: *school_id,
        })
    }
}