# API Development Rules and Guidelines
## Goddard School Enrollment System Backend

### Table of Contents
1. [Project Structure](#project-structure)
2. [API Endpoint Naming Conventions](#api-endpoint-naming-conventions)
3. [Database Connection Management](#database-connection-management)
4. [Authorization and Authentication](#authorization-and-authentication)
5. [Error Handling](#error-handling)
6. [Code Reusability and Utils](#code-reusability-and-utils)
7. [Model and Schema Management](#model-and-schema-management)
8. [Service Layer Architecture](#service-layer-architecture)
9. [Data Access Object (DAO) Pattern](#data-access-object-dao-pattern)
10. [Configuration Management](#configuration-management)
11. [Validation Rules](#validation-rules)
12. [Testing Guidelines](#testing-guidelines)

---

## 1. Project Structure

### Mandatory Folder Structure
```
src/
├── controllers/           # HTTP handlers and route logic
│   ├── school_controller.rs
│   ├── user_controller.rs
│   ├── classroom_controller.rs
│   ├── form_controller.rs
│   └── enrollment_controller.rs
├── services/             # Business logic layer
│   ├── school_service.rs
│   ├── user_service.rs
│   ├── classroom_service.rs
│   ├── form_service.rs
│   └── enrollment_service.rs
├── dao/                  # Data Access Objects
│   ├── school_dao.rs
│   ├── user_dao.rs
│   ├── classroom_dao.rs
│   ├── form_dao.rs
│   └── enrollment_dao.rs
├── models/               # Data structures and schemas
│   ├── school.rs
│   ├── user.rs
│   ├── classroom.rs
│   ├── form.rs
│   └── enrollment.rs
├── utils/                # Reusable utility functions
│   ├── auth.rs           # Authentication utilities
│   ├── validation.rs     # Input validation utilities
│   ├── response.rs       # Response formatting utilities
│   ├── date_time.rs      # Date/time utilities
│   └── crypto.rs         # Encryption/hashing utilities
├── config/               # Configuration management
│   ├── database.rs       # Database configuration
│   ├── app_config.rs     # Application configuration
│   └── email_config.rs   # Email service configuration
├── middleware/           # HTTP middleware
│   ├── auth_middleware.rs
│   ├── cors_middleware.rs
│   └── logging_middleware.rs
├── error/                # Centralized error handling
│   ├── error_types.rs    # Error type definitions
│   ├── error_handler.rs  # Global error handler
│   └── validation_error.rs
└── main.rs              # Application entry point
```

### Rules:
1. **One responsibility per file**: Each file should handle only one domain/entity
2. **No cross-layer dependencies**: Controllers cannot directly access DAOs
3. **Consistent naming**: Use snake_case for files, PascalCase for structs/enums
4. **Modular structure**: Each module should be self-contained and testable

---

## 2. API Endpoint Naming Conventions

### Endpoint Structure Rules
```
HTTP_METHOD /resource[/{id}][/sub-resource][?query-params]
```

### Naming Standards

#### ✅ Correct Examples:
```
GET    /schools                          # List all schools
POST   /schools                          # Create new school
GET    /schools/{id}                     # Get specific school
PUT    /schools/{id}                     # Update specific school
DELETE /schools/{id}                     # Delete specific school

GET    /schools/{id}/classrooms          # Get classrooms for a school
POST   /schools/{id}/classrooms          # Create classroom in school

GET    /enrollments/children-forms       # Get enrollment children with forms
GET    /form-templates/by-status         # Get forms grouped by status
```

#### ❌ Incorrect Examples:
```
GET    /getSchools                       # No verbs in resource names
POST   /school/create                    # No action words in URLs
GET    /schools/list                     # Redundant with HTTP method
PUT    /updateSchool/{id}                # Action word not needed
```

### Rules:
1. **Use nouns, not verbs** for resource names
2. **Use plural nouns** for collections (`/schools`, not `/school`)
3. **Use kebab-case** for multi-word resources (`/form-templates`)
4. **Use query parameters** for filtering (`?school_id=uuid&status=active`)
5. **Use path parameters** for resource identification (`/schools/{id}`)
6. **Maximum 3 levels** of nesting in URLs

---

## 3. Database Connection Management

### Centralized Connection Pool

#### File: `src/config/database.rs`
```rust
use sqlx::{Pool, Postgres, PgPool};
use std::env;

pub struct DatabaseConfig {
    pub pool: PgPool,
}

impl DatabaseConfig {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let pool = PgPool::connect(&database_url).await?;

        Ok(DatabaseConfig { pool })
    }

    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

// Global database instance
pub static DB_POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn initialize_database() -> Result<(), sqlx::Error> {
    let config = DatabaseConfig::new().await?;
    DB_POOL.set(config.pool).map_err(|_| {
        sqlx::Error::Configuration("Failed to set database pool".into())
    })?;
    Ok(())
}

pub fn get_db_pool() -> &'static PgPool {
    DB_POOL.get().expect("Database pool not initialized")
}
```

### Rules:
1. **Single connection pool** shared across the application
2. **Initialize once** at application startup
3. **Use dependency injection** to pass pool to DAOs
4. **Handle connection errors** gracefully with retries
5. **Connection pool configuration** via environment variables

---

## 4. Authorization and Authentication

### Centralized Auth System

#### File: `src/utils/auth.rs`
```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub school_id: Option<String>,
    pub exp: usize,
}

pub struct AuthService {
    jwt_secret: String,
    api_key: String,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            api_key: env::var("OWNER_API_KEY").expect("OWNER_API_KEY must be set"),
        }
    }

    pub fn validate_jwt_token(&self, token: &str) -> Result<Claims, AuthError> {
        let validation = Validation::new(Algorithm::HS256);
        let key = DecodingKey::from_secret(self.jwt_secret.as_ref());

        decode::<Claims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(AuthError::InvalidToken)
    }

    pub fn validate_api_key(&self, provided_key: &str) -> Result<(), AuthError> {
        if provided_key == self.api_key {
            Ok(())
        } else {
            Err(AuthError::InvalidApiKey)
        }
    }

    pub fn check_role_permission(&self, user_role: &str, required_roles: &[&str]) -> Result<(), AuthError> {
        if required_roles.contains(&user_role) || user_role == "SuperAdmin" {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }

    pub fn check_school_access(&self, jwt_school_id: Option<&str>, requested_school_id: &str, user_role: &str) -> Result<(), AuthError> {
        if user_role == "SuperAdmin" {
            return Ok(());
        }

        match jwt_school_id {
            Some(school_id) if school_id == requested_school_id => Ok(()),
            _ => Err(AuthError::SchoolAccessDenied),
        }
    }
}
```

#### File: `src/middleware/auth_middleware.rs`
```rust
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn jwt_auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth_service = AuthService::new();

    let token = extract_bearer_token(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "Missing authorization header".to_string()))?;

    let claims = auth_service.validate_jwt_token(&token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;

    // Add claims to request extensions for use in controllers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

pub async fn api_key_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth_service = AuthService::new();

    let api_key = headers.get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing API key".to_string()))?;

    auth_service.validate_api_key(api_key)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;

    Ok(next.run(request).await)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers.get("authorization")?
        .to_str().ok()?
        .strip_prefix("Bearer ")
}
```

### Rules:
1. **Single auth service** for all authentication logic
2. **Middleware-based authentication** for route protection
3. **Claims extraction** available in all protected routes
4. **Role-based permissions** with hierarchical access
5. **School-scoped access control** for multi-tenant security

---

## 5. Error Handling

### Centralized Error System

#### File: `src/error/error_types.rs`
```rust
use serde::Serialize;
use axum::http::StatusCode;

#[derive(Debug, thiserror::Error, Serialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: String,
    pub timestamp: String,
}

impl AppError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Database(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_string(),
            message: self.user_message(),
            code: self.error_code(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn user_message(&self) -> String {
        match self {
            AppError::Validation(msg) => msg.clone(),
            AppError::Authentication(_) => "Authentication failed".to_string(),
            AppError::Authorization(_) => "Access denied".to_string(),
            AppError::NotFound(resource) => format!("{} not found", resource),
            AppError::Conflict(msg) => msg.clone(),
            _ => "An internal error occurred".to_string(),
        }
    }

    fn error_code(&self) -> String {
        match self {
            AppError::Database(_) => "DATABASE_ERROR".to_string(),
            AppError::Validation(_) => "VALIDATION_ERROR".to_string(),
            AppError::Authentication(_) => "AUTH_ERROR".to_string(),
            AppError::Authorization(_) => "AUTHORIZATION_ERROR".to_string(),
            AppError::NotFound(_) => "NOT_FOUND".to_string(),
            AppError::Conflict(_) => "CONFLICT".to_string(),
            AppError::ExternalService(_) => "EXTERNAL_SERVICE_ERROR".to_string(),
            AppError::Internal(_) => "INTERNAL_ERROR".to_string(),
        }
    }
}
```

#### File: `src/error/error_handler.rs`
```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.to_status_code();
        let error_response = self.to_error_response();

        // Log error for monitoring
        log_error(&self, status);

        (status, Json(error_response)).into_response()
    }
}

fn log_error(error: &AppError, status: StatusCode) {
    match status {
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::BAD_GATEWAY => {
            tracing::error!("Error: {} - Status: {}", error, status);
        }
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            tracing::warn!("Client error: {} - Status: {}", error, status);
        }
        _ => {
            tracing::info!("Request error: {} - Status: {}", error, status);
        }
    }
}

// Result type alias for convenience
pub type ApiResult<T> = Result<T, AppError>;
```

### Rules:
1. **Single error enum** for all application errors
2. **Consistent error responses** with structured format
3. **Automatic logging** based on error severity
4. **User-friendly messages** without exposing internal details
5. **HTTP status code mapping** for proper REST responses

---

## 6. Code Reusability and Utils

### Common Utilities

#### File: `src/utils/validation.rs`
```rust
use uuid::Uuid;
use validator::{Validate, ValidationError};

pub struct ValidationUtils;

impl ValidationUtils {
    pub fn validate_uuid(uuid_str: &str) -> Result<Uuid, AppError> {
        Uuid::parse_str(uuid_str)
            .map_err(|_| AppError::Validation(format!("Invalid UUID format: {}", uuid_str)))
    }

    pub fn validate_email(email: &str) -> Result<(), AppError> {
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .unwrap();

        if email_regex.is_match(email) {
            Ok(())
        } else {
            Err(AppError::Validation("Invalid email format".to_string()))
        }
    }

    pub fn validate_role(role: &str) -> Result<(), AppError> {
        match role {
            "SuperAdmin" | "Admin" | "Teacher" | "Parent" => Ok(()),
            _ => Err(AppError::Validation("Invalid role".to_string())),
        }
    }

    pub fn validate_school_access(
        jwt_school_id: Option<&str>,
        requested_school_id: &str,
        user_role: &str,
    ) -> Result<(), AppError> {
        if user_role == "SuperAdmin" {
            return Ok(());
        }

        match jwt_school_id {
            Some(school_id) if school_id == requested_school_id => Ok(()),
            _ => Err(AppError::Authorization("School access denied".to_string())),
        }
    }
}
```

#### File: `src/utils/response.rs`
```rust
use serde::Serialize;
use axum::{http::StatusCode, response::IntoResponse, Json};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

pub struct ResponseUtils;

impl ResponseUtils {
    pub fn success<T: Serialize>(data: T) -> impl IntoResponse {
        let response = ApiResponse {
            success: true,
            data: Some(data),
            message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        (StatusCode::OK, Json(response))
    }

    pub fn created<T: Serialize>(data: T) -> impl IntoResponse {
        let response = ApiResponse {
            success: true,
            data: Some(data),
            message: Some("Resource created successfully".to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        (StatusCode::CREATED, Json(response))
    }

    pub fn no_content() -> impl IntoResponse {
        StatusCode::NO_CONTENT
    }

    pub fn paginated<T: Serialize>(
        data: Vec<T>,
        page: u32,
        per_page: u32,
        total: u64,
    ) -> impl IntoResponse {
        let total_pages = (total as f64 / per_page as f64).ceil() as u32;

        let response = PaginatedResponse {
            data,
            pagination: PaginationInfo {
                page,
                per_page,
                total,
                total_pages,
            },
        };

        (StatusCode::OK, Json(response))
    }
}
```

### Rules:
1. **Create utility modules** for repeated functionality
2. **Generic implementations** where possible
3. **Consistent return types** across similar functions
4. **Input validation utilities** for common patterns
5. **Response formatting utilities** for API consistency

---

## 7. Model and Schema Management

### File: `src/models/school.rs`
```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct School {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
    pub settings: serde_json::Value,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSchoolRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(length(min = 1, max = 100), regex = "^[a-z0-9-]+$")]
    pub subdomain: String,

    pub settings: Option<SchoolSettings>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSchoolRequest {
    pub id: Uuid,

    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 100), regex = "^[a-z0-9-]+$")]
    pub subdomain: Option<String>,

    pub settings: Option<SchoolSettings>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SchoolSettings {
    pub timezone: String,
    pub max_enrollment: Option<u32>,
    pub age_groups: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SchoolResponse {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
    pub settings: SchoolSettings,
    pub created_at: DateTime<Utc>,
}

impl From<School> for SchoolResponse {
    fn from(school: School) -> Self {
        Self {
            id: school.id,
            name: school.name,
            subdomain: school.subdomain,
            settings: serde_json::from_value(school.settings).unwrap_or_default(),
            created_at: school.created_at,
        }
    }
}
```

### Rules:
1. **Separate models** for database, request, and response
2. **Validation attributes** on request models
3. **Consistent field naming** across related models
4. **Type safety** with proper Rust types
5. **Conversion implementations** between model types

---

## 8. Service Layer Architecture

### File: `src/services/school_service.rs`
```rust
use crate::{
    dao::school_dao::SchoolDao,
    models::school::*,
    error::{AppError, ApiResult},
    utils::validation::ValidationUtils,
};

pub struct SchoolService {
    dao: SchoolDao,
}

impl SchoolService {
    pub fn new(dao: SchoolDao) -> Self {
        Self { dao }
    }

    pub async fn create_school(&self, request: CreateSchoolRequest) -> ApiResult<SchoolResponse> {
        // Validate input
        request.validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check subdomain uniqueness
        if self.dao.subdomain_exists(&request.subdomain).await? {
            return Err(AppError::Conflict("Subdomain already exists".to_string()));
        }

        // Create school
        let school = self.dao.create_school(request).await?;

        Ok(school.into())
    }

    pub async fn get_all_schools(&self) -> ApiResult<Vec<SchoolResponse>> {
        let schools = self.dao.get_all_active_schools().await?;
        let responses = schools.into_iter().map(|s| s.into()).collect();

        Ok(responses)
    }

    pub async fn get_school_by_id(&self, school_id: Uuid) -> ApiResult<SchoolResponse> {
        let school = self.dao.get_school_by_id(school_id).await?
            .ok_or_else(|| AppError::NotFound("School".to_string()))?;

        Ok(school.into())
    }

    pub async fn update_school(&self, request: UpdateSchoolRequest) -> ApiResult<SchoolResponse> {
        // Validate input
        request.validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check if school exists
        if !self.dao.school_exists(request.id).await? {
            return Err(AppError::NotFound("School".to_string()));
        }

        // Check subdomain uniqueness if being updated
        if let Some(ref subdomain) = request.subdomain {
            if self.dao.subdomain_exists_excluding_school(subdomain, request.id).await? {
                return Err(AppError::Conflict("Subdomain already exists".to_string()));
            }
        }

        let school = self.dao.update_school(request).await?;

        Ok(school.into())
    }

    pub async fn delete_school(&self, school_id: Uuid) -> ApiResult<()> {
        if !self.dao.school_exists(school_id).await? {
            return Err(AppError::NotFound("School".to_string()));
        }

        self.dao.soft_delete_school(school_id).await?;

        Ok(())
    }
}
```

### Rules:
1. **Business logic only** in service layer
2. **Input validation** before processing
3. **Error handling** with proper error types
4. **DAO dependency injection** for data access
5. **Return response models** not database models

---

## 9. Data Access Object (DAO) Pattern

### File: `src/dao/school_dao.rs`
```rust
use sqlx::PgPool;
use uuid::Uuid;
use crate::{
    models::school::*,
    error::{AppError, ApiResult},
};

pub struct SchoolDao {
    pool: PgPool,
}

impl SchoolDao {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_school(&self, request: CreateSchoolRequest) -> ApiResult<School> {
        let settings_json = serde_json::to_value(&request.settings)
            .map_err(|e| AppError::Internal(format!("JSON serialization error: {}", e)))?;

        let school = sqlx::query_as!(
            School,
            r#"
            INSERT INTO schools (id, name, subdomain, settings, created_at)
            VALUES (gen_random_uuid(), $1, $2, $3, NOW())
            RETURNING id, name, subdomain, settings, is_active, created_at, updated_at
            "#,
            request.name,
            request.subdomain,
            settings_json
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(school)
    }

    pub async fn get_all_active_schools(&self) -> ApiResult<Vec<School>> {
        let schools = sqlx::query_as!(
            School,
            r#"
            SELECT id, name, subdomain, settings, is_active, created_at, updated_at
            FROM schools
            WHERE (is_active = true OR is_active IS NULL)
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(schools)
    }

    pub async fn get_school_by_id(&self, school_id: Uuid) -> ApiResult<Option<School>> {
        let school = sqlx::query_as!(
            School,
            r#"
            SELECT id, name, subdomain, settings, is_active, created_at, updated_at
            FROM schools
            WHERE id = $1 AND (is_active = true OR is_active IS NULL)
            "#,
            school_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(school)
    }

    pub async fn subdomain_exists(&self, subdomain: &str) -> ApiResult<bool> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM schools WHERE subdomain = $1 AND (is_active = true OR is_active IS NULL)",
            subdomain
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count.unwrap_or(0) > 0)
    }

    pub async fn school_exists(&self, school_id: Uuid) -> ApiResult<bool> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM schools WHERE id = $1 AND (is_active = true OR is_active IS NULL)",
            school_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count.unwrap_or(0) > 0)
    }

    pub async fn soft_delete_school(&self, school_id: Uuid) -> ApiResult<()> {
        sqlx::query!(
            "UPDATE schools SET is_active = false, updated_at = NOW() WHERE id = $1",
            school_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
```

### Rules:
1. **Database operations only** in DAO layer
2. **Proper error conversion** from sqlx to AppError
3. **Query optimization** with appropriate indexes
4. **Transaction support** for complex operations
5. **Type-safe queries** using sqlx macros

---

## 10. Configuration Management

### File: `src/config/app_config.rs`
```rust
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub email: EmailConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration: u64,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    pub resend_api_key: String,
    pub from_email: String,
    pub template_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(AppConfig {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8000".to_string())
                    .parse()
                    .map_err(|_| ConfigError::InvalidPort)?,
                cors_origins: env::var("CORS_ORIGINS")
                    .unwrap_or_else(|_| "*".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .map_err(|_| ConfigError::MissingEnvironmentVariable("DATABASE_URL"))?,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
                connection_timeout: env::var("DB_CONNECTION_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
            auth: AuthConfig {
                jwt_secret: env::var("JWT_SECRET")
                    .map_err(|_| ConfigError::MissingEnvironmentVariable("JWT_SECRET"))?,
                jwt_expiration: env::var("JWT_EXPIRATION")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .unwrap_or(86400),
                api_key: env::var("OWNER_API_KEY")
                    .map_err(|_| ConfigError::MissingEnvironmentVariable("OWNER_API_KEY"))?,
            },
            email: EmailConfig {
                resend_api_key: env::var("RESEND_API_KEY")
                    .map_err(|_| ConfigError::MissingEnvironmentVariable("RESEND_API_KEY"))?,
                from_email: env::var("FROM_EMAIL")
                    .unwrap_or_else(|_| "noreply@goddardschool.com".to_string()),
                template_path: env::var("EMAIL_TEMPLATE_PATH")
                    .unwrap_or_else(|_| "./templates".to_string()),
            },
            logging: LoggingConfig {
                level: env::var("LOG_LEVEL")
                    .unwrap_or_else(|_| "info".to_string()),
                file_path: env::var("LOG_FILE").ok(),
            },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvironmentVariable(&'static str),

    #[error("Invalid port configuration")]
    InvalidPort,
}
```

### Rules:
1. **Environment-based configuration** for different environments
2. **Validation** of configuration values at startup
3. **Default values** for optional settings
4. **Type-safe configuration** structs
5. **Error handling** for missing/invalid config

---

## 11. Validation Rules

### Request Validation Standards
```rust
use validator::{Validate, ValidationError};

// Field-level validation rules
#[derive(Validate)]
pub struct UserRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 2, max = 50))]
    pub first_name: String,

    #[validate(length(min = 2, max = 50))]
    pub last_name: String,

    #[validate(custom = "validate_role")]
    pub role: String,

    #[validate(custom = "validate_uuid")]
    pub school_id: String,
}

fn validate_role(role: &str) -> Result<(), ValidationError> {
    match role {
        "SuperAdmin" | "Admin" | "Teacher" | "Parent" => Ok(()),
        _ => Err(ValidationError::new("invalid_role")),
    }
}

fn validate_uuid(uuid: &str) -> Result<(), ValidationError> {
    uuid::Uuid::parse_str(uuid)
        .map(|_| ())
        .map_err(|_| ValidationError::new("invalid_uuid"))
}
```

### Rules:
1. **Validate all input** at the request level
2. **Custom validators** for business-specific rules
3. **Consistent error messages** for validation failures
4. **Type coercion** where appropriate
5. **Sanitization** of input data

---

## 12. Testing Guidelines

### File: `tests/integration/school_test.rs`
```rust
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_create_school_success() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "name": "Test School",
        "subdomain": "test-school",
        "settings": {
            "timezone": "UTC",
            "max_enrollment": 100,
            "age_groups": ["infant", "toddler"]
        }
    });

    let response = server
        .post("/schools")
        .add_header("X-API-Key", "test-api-key")
        .json(&request_body)
        .await;

    response.assert_status_created();

    let school: serde_json::Value = response.json();
    assert_eq!(school["name"], "Test School");
    assert_eq!(school["subdomain"], "test-school");
}

#[tokio::test]
async fn test_create_school_invalid_api_key() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/schools")
        .add_header("X-API-Key", "invalid-key")
        .json(&json!({}))
        .await;

    response.assert_status_unauthorized();
}
```

### Rules:
1. **Integration tests** for API endpoints
2. **Unit tests** for service and utility functions
3. **Test database** separate from development
4. **Mock external services** for isolated testing
5. **Comprehensive test coverage** for critical paths

---

## Implementation Checklist

### Phase 1: Foundation
- [ ] Set up project structure according to guidelines
- [ ] Implement centralized database connection
- [ ] Create error handling system
- [ ] Set up authentication middleware
- [ ] Create basic utility functions

### Phase 2: Core Services
- [ ] Implement School management APIs
- [ ] Implement User management APIs
- [ ] Implement Classroom management APIs
- [ ] Add proper validation and error handling
- [ ] Create comprehensive tests

### Phase 3: Advanced Features
- [ ] Implement Form management APIs
- [ ] Implement Enrollment management APIs
- [ ] Add audit logging
- [ ] Implement rate limiting
- [ ] Add monitoring and metrics

### Phase 4: Production Readiness
- [ ] Performance optimization
- [ ] Security hardening
- [ ] Documentation completion
- [ ] Deployment configuration
- [ ] Monitoring setup

---

## Enforcement Rules

1. **Code Reviews**: All code must follow these guidelines
2. **Automated Checks**: Use linting and formatting tools
3. **Testing Requirements**: Minimum 80% test coverage
4. **Documentation**: All public APIs must be documented
5. **Performance**: Response times under 200ms for standard operations

This document should be updated as the project evolves and new patterns emerge.