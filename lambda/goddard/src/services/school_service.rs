use crate::{
    dao::SchoolDao,
    error::{AppError, ApiResult},
    models::school::{
        CreateSchoolRequest, UpdateSchoolRequest,
        SchoolResponse, SchoolListItem, DeleteSchoolResponse,
    },
};
use uuid::Uuid;
use std::time::Duration;

pub struct SchoolService {
    dao: SchoolDao,
}

impl SchoolService {
    pub fn new(dao: SchoolDao) -> Self {
        Self { dao }
    }

    pub async fn create_school(&self, request: CreateSchoolRequest) -> ApiResult<SchoolResponse> {
        println!("[SchoolService] Starting create_school with request: name={}, subdomain={}", request.name, request.subdomain);

        // Validate input
        if request.name.trim().is_empty() {
            println!("[SchoolService] Validation failed: empty school name");
            return Err(AppError::Validation("School name cannot be empty".to_string()));
        }

        if request.subdomain.trim().is_empty() {
            println!("[SchoolService] Validation failed: empty subdomain");
            return Err(AppError::Validation("Subdomain cannot be empty".to_string()));
        }

        // Validate subdomain format (alphanumeric and hyphens only)
        if !request.subdomain.chars().all(|c| c.is_alphanumeric() || c == '-') {
            println!("[SchoolService] Validation failed: invalid subdomain format");
            return Err(AppError::Validation("Subdomain can only contain letters, numbers, and hyphens".to_string()));
        }

        println!("[SchoolService] Input validation passed, checking subdomain existence");

        // Check if subdomain already exists with timeout
        let timeout_duration = Duration::from_secs(10);
        let subdomain_check = self.dao.check_subdomain_exists(&request.subdomain, None);

        println!("[SchoolService] Starting subdomain check with 10s timeout");
        let subdomain_exists = match tokio::time::timeout(timeout_duration, subdomain_check).await {
            Ok(Ok(exists)) => {
                println!("[SchoolService] Subdomain check completed: exists={}", exists);
                exists
            },
            Ok(Err(e)) => {
                println!("[SchoolService] Subdomain check failed with error: {:?}", e);
                return Err(e);
            },
            Err(_) => {
                println!("[SchoolService] Subdomain check timed out after 10s");
                return Err(AppError::Database("Database operation timeout (10s) during subdomain check - please try again".to_string()))
            }
        };

        if subdomain_exists {
            println!("[SchoolService] Subdomain already exists: {}", request.subdomain);
            return Err(AppError::Conflict("Subdomain already exists".to_string()));
        }

        println!("[SchoolService] Subdomain available, starting school creation with 10s timeout");

        // Create school with timeout
        let create_operation = self.dao.create_school(&request);
        match tokio::time::timeout(timeout_duration, create_operation).await {
            Ok(Ok(school)) => {
                println!("[SchoolService] School created successfully: id={}", school.id);
                Ok(SchoolResponse::from(school))
            },
            Ok(Err(e)) => {
                println!("[SchoolService] School creation failed with error: {:?}", e);
                Err(e)
            },
            Err(_) => {
                println!("[SchoolService] School creation timed out after 10s");
                Err(AppError::Database("Database operation timeout (10s) during school creation - please try again".to_string()))
            }
        }
    }

    pub async fn get_all_schools(&self) -> ApiResult<Vec<SchoolListItem>> {
        let timeout_duration = Duration::from_secs(10);
        let operation = self.dao.get_all_schools();

        match tokio::time::timeout(timeout_duration, operation).await {
            Ok(Ok(schools)) => {
                let school_list = schools.into_iter().map(SchoolListItem::from).collect();
                Ok(school_list)
            },
            Ok(Err(e)) => Err(e),
            Err(_) => Err(AppError::Database("Database operation timeout (10s) - please try again".to_string()))
        }
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