use crate::{
    dao::{SchoolDao, AuthDao},
    error::{AppError, ApiResult},
    models::school::{
        CreateSchoolRequest, UpdateSchoolRequest,
        SchoolResponse, SchoolListItem, DeleteSchoolResponse,
        CreateSchoolWithOwnerRequest, CreateSchoolWithOwnerResponse, OwnerCreatedResponse,
        SchoolWithOwnerResponse, OwnerDetailsResponse,
    },
    services::{SupabaseClient, supabase_client::UserMetadata},
    utils::ValidationUtils,
};
use uuid::Uuid;
use std::time::Duration;

pub struct SchoolService {
    dao: SchoolDao,
    supabase_client: SupabaseClient,
    auth_dao: AuthDao,
}

impl SchoolService {
    pub fn new(dao: SchoolDao, supabase_client: SupabaseClient, auth_dao: AuthDao) -> Self {
        Self { dao, supabase_client, auth_dao }
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

    pub async fn create_school_with_owner(
        &self,
        request: CreateSchoolWithOwnerRequest
    ) -> ApiResult<CreateSchoolWithOwnerResponse> {
        println!("[SchoolService] Starting create_school_with_owner");
        println!("[SchoolService] School: name={}, subdomain={}", request.school.name, request.school.subdomain);
        println!("[SchoolService] Owner: email={}, first_name={}, last_name={}", request.owner.email, request.owner.first_name, request.owner.last_name);

        // STEP 1: Validate school fields
        if request.school.name.trim().is_empty() {
            println!("[SchoolService] Validation failed: empty school name");
            return Err(AppError::Validation("School name cannot be empty".to_string()));
        }

        if request.school.subdomain.trim().is_empty() {
            println!("[SchoolService] Validation failed: empty subdomain");
            return Err(AppError::Validation("Subdomain cannot be empty".to_string()));
        }

        // Validate subdomain format (alphanumeric and hyphens only)
        if !request.school.subdomain.chars().all(|c| c.is_alphanumeric() || c == '-') {
            println!("[SchoolService] Validation failed: invalid subdomain format");
            return Err(AppError::Validation("Subdomain can only contain letters, numbers, and hyphens".to_string()));
        }

        // STEP 2: Validate owner fields
        ValidationUtils::validate_email(&request.owner.email)?;

        if request.owner.first_name.trim().is_empty() {
            println!("[SchoolService] Validation failed: empty owner first name");
            return Err(AppError::Validation("Owner first name cannot be empty".to_string()));
        }

        if request.owner.last_name.trim().is_empty() {
            println!("[SchoolService] Validation failed: empty owner last name");
            return Err(AppError::Validation("Owner last name cannot be empty".to_string()));
        }

        println!("[SchoolService] All validation passed");

        // STEP 3: Check subdomain availability with timeout
        let timeout_duration = Duration::from_secs(10);
        let subdomain_check = self.dao.check_subdomain_exists(&request.school.subdomain, None);

        println!("[SchoolService] Checking subdomain availability with 10s timeout");
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
            println!("[SchoolService] Subdomain already exists: {}", request.school.subdomain);
            return Err(AppError::Conflict("Subdomain already exists".to_string()));
        }

        // STEP 4: Create school with timeout
        let create_school_request = CreateSchoolRequest {
            name: request.school.name.clone(),
            subdomain: request.school.subdomain.clone(),
            settings: request.school.settings.clone(),
        };

        println!("[SchoolService] Creating school with 10s timeout");
        let create_operation = self.dao.create_school(&create_school_request);
        let school = match tokio::time::timeout(timeout_duration, create_operation).await {
            Ok(Ok(school)) => {
                println!("[SchoolService] School created successfully: id={}", school.id);
                school
            },
            Ok(Err(e)) => {
                println!("[SchoolService] School creation failed with error: {:?}", e);
                return Err(e);
            },
            Err(_) => {
                println!("[SchoolService] School creation timed out after 10s");
                return Err(AppError::Database("Database operation timeout (10s) during school creation - please try again".to_string()))
            }
        };

        // STEP 5: Create SuperAdmin user with school_id in metadata
        println!("[SchoolService] Creating SuperAdmin user for school_id={}", school.id);
        let metadata = UserMetadata::new(
            Some(school.id),
            Some(request.owner.first_name.clone()),
            Some(request.owner.last_name.clone()),
            Some("SuperAdmin".to_string()),
            request.owner.phone_number.clone(),
            Some(true),  // is_verified = true
        )
        .with_school_name(school.name.clone());  // ADD school name for email personalization

        let user_result = self.supabase_client
            .create_user_invitation_enhanced(&request.owner.email, metadata)
            .await;

        // STEP 6: Handle result - ROLLBACK if user creation fails
        match user_result {
            Ok(user_id) => {
                println!("[SchoolService] SuperAdmin user created successfully: user_id={}", user_id);
                println!("[SchoolService] School and owner creation completed successfully");

                Ok(CreateSchoolWithOwnerResponse {
                    school: SchoolResponse::from(school),
                    owner: OwnerCreatedResponse {
                        user_id,
                        email: request.owner.email,
                        first_name: request.owner.first_name,
                        last_name: request.owner.last_name,
                        role: "SuperAdmin".to_string(),
                        email_sent: true,
                        message: "School and SuperAdmin created successfully. Confirmation email sent.".to_string(),
                    }
                })
            },
            Err(e) => {
                // ROLLBACK: Delete the school we just created
                println!("[SchoolService] User creation failed, rolling back school: {}", school.id);
                let rollback_result = self.dao.delete_school(&school.id).await;
                match rollback_result {
                    Ok(_) => println!("[SchoolService] School rollback successful"),
                    Err(rollback_err) => println!("[SchoolService] School rollback failed (non-critical): {:?}", rollback_err),
                }

                Err(AppError::ExternalService(
                    format!("Failed to create SuperAdmin user: {}. School creation rolled back.", e)
                ))
            }
        }
    }

    pub async fn get_school_with_owner(
        &self,
        school_id: &Uuid
    ) -> ApiResult<SchoolWithOwnerResponse> {
        println!("[SchoolService] Getting school with owner for school_id: {}", school_id);

        // STEP 1: Get school by ID
        let school = self.dao.get_school_by_id(school_id).await?
            .ok_or_else(|| {
                println!("[SchoolService] School not found: {}", school_id);
                AppError::NotFound("School not found".to_string())
            })?;

        println!("[SchoolService] School found: {}", school.name);

        // STEP 2: Get SuperAdmin owner with login status from auth_dao
        let owner = self.auth_dao.get_superadmin_with_login_status_by_school(school_id).await?
            .ok_or_else(|| {
                println!("[SchoolService] SuperAdmin owner not found for school: {}", school_id);
                AppError::NotFound("SuperAdmin owner not found for this school".to_string())
            })?;

        println!("[SchoolService] SuperAdmin found: {}, has_logged_in: {}",
            owner.email, owner.last_sign_in_at.is_some());

        // STEP 3: Build response
        Ok(SchoolWithOwnerResponse {
            school: SchoolResponse::from(school),
            owner: OwnerDetailsResponse {
                user_id: owner.id,
                email: owner.email,
                first_name: owner.first_name,
                last_name: owner.last_name,
                phone_number: owner.phone_number,
                role: owner.role,
                is_verified: owner.is_verified,
                has_logged_in: owner.last_sign_in_at.is_some(),
                last_sign_in_at: owner.last_sign_in_at,
                created_at: owner.created_at,
            }
        })
    }

    pub async fn get_all_schools_with_owners(&self) -> ApiResult<Vec<SchoolWithOwnerResponse>> {
        println!("[SchoolService] Getting all schools with owners");

        // Step 1: Get all schools
        let schools = self.dao.get_all_schools().await?;
        println!("[SchoolService] Found {} schools", schools.len());

        // Step 2: For each school, get SuperAdmin owner
        let mut results = Vec::new();

        for school in schools {
            println!("[SchoolService] Processing school: {} ({})", school.name, school.id);

            // Get SuperAdmin owner for this school
            match self.auth_dao.get_superadmin_with_login_status_by_school(&school.id).await? {
                Some(owner) => {
                    println!("[SchoolService] Found SuperAdmin for school {}: {}", school.name, owner.email);

                    results.push(SchoolWithOwnerResponse {
                        school: SchoolResponse::from(school),
                        owner: OwnerDetailsResponse {
                            user_id: owner.id,
                            email: owner.email,
                            first_name: owner.first_name,
                            last_name: owner.last_name,
                            phone_number: owner.phone_number,
                            role: owner.role,
                            is_verified: owner.is_verified,
                            has_logged_in: owner.last_sign_in_at.is_some(),
                            last_sign_in_at: owner.last_sign_in_at,
                            created_at: owner.created_at,
                        }
                    });
                },
                None => {
                    // School exists but has no SuperAdmin owner - skip it or log warning
                    println!("[SchoolService] WARNING: School {} has no SuperAdmin owner", school.name);
                    // DECISION: Skip schools without SuperAdmin (don't include in results)
                }
            }
        }

        println!("[SchoolService] Returning {} schools with owners", results.len());
        Ok(results)
    }
}