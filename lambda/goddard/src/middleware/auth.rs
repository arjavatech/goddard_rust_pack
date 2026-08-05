use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware::Next,
    response::{Response, IntoResponse},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;
use crate::models::schema::UserRole;
use crate::error::error_types::AppError;
use reqwest::Client;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    success: bool,
    message: String,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub role: UserRole,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

// Supabase JWT user metadata structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseUserMetadata {
    pub email: String,
    pub email_verified: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_verified: Option<bool>,
    pub role: String,
    pub school_id: String,
    pub sub: String,
}

// Supabase JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseJwtClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub email: String,
    pub user_metadata: SupabaseUserMetadata,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub role: UserRole,
    pub email: String,
}

pub async fn api_key_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok());

    if api_key.is_none() {
        let error_response = ErrorResponse {
            success: false,
            message: "API key is required. Please provide X-API-Key header".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    let owner_api_key = match env::var("OWNER_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            let error_response = ErrorResponse {
                success: false,
                message: "Server configuration error".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response());
        }
    };

    if api_key.unwrap() != owner_api_key {
        let error_response = ErrorResponse {
            success: false,
            message: "Invalid API key".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    Ok(next.run(request).await)
}

pub async fn jwt_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if auth_header.is_none() {
        let error_response = ErrorResponse {
            success: false,
            message: "Authorization header is required".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    let auth_header = auth_header.unwrap();
    if !auth_header.starts_with("Bearer ") {
        let error_response = ErrorResponse {
            success: false,
            message: "Invalid authorization format. Use: Bearer <token>".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    let token = &auth_header[7..];

    let jwt_secret = match env::var("JWT_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            let error_response = ErrorResponse {
                success: false,
                message: "Server configuration error".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response());
        }
    };

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["goddard-school"]);
    validation.set_issuer(&["goddard-auth"]);

    let token_data = match decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    ) {
        Ok(data) => data,
        Err(e) => {
            let error_response = ErrorResponse {
                success: false,
                message: format!("Invalid or expired token: {}", e),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
        }
    };

    let auth_context = AuthContext {
        user_id: token_data.claims.user_id,
        school_id: token_data.claims.school_id,
        role: token_data.claims.role,
        email: token_data.claims.email,
    };

    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}

/// Admin-only middleware - allows only Admin and SuperAdmin roles
pub async fn jwt_admin_only(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // First validate JWT and extract auth context
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if auth_header.is_none() {
        let error_response = ErrorResponse {
            success: false,
            message: "Authorization header is required".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    let auth_header = auth_header.unwrap();
    if !auth_header.starts_with("Bearer ") {
        let error_response = ErrorResponse {
            success: false,
            message: "Invalid authorization format. Use: Bearer <token>".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    let token = &auth_header[7..];

    let jwt_secret = match env::var("JWT_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            let error_response = ErrorResponse {
                success: false,
                message: "Server configuration error".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response());
        }
    };

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["goddard-school"]);
    validation.set_issuer(&["goddard-auth"]);

    let token_data = match decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    ) {
        Ok(data) => data,
        Err(e) => {
            let error_response = ErrorResponse {
                success: false,
                message: format!("Invalid or expired token: {}", e),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
        }
    };

    // Check if user has admin privileges
    match token_data.claims.role {
        UserRole::Admin | UserRole::SuperAdmin => {
            // Admin or SuperAdmin - create auth context and allow access
            let auth_context = AuthContext {
                user_id: token_data.claims.user_id,
                school_id: token_data.claims.school_id,
                role: token_data.claims.role,
                email: token_data.claims.email,
            };

            request.extensions_mut().insert(auth_context);
            Ok(next.run(request).await)
        }
        _ => {
            let error_response = ErrorResponse {
                success: false,
                message: "Access denied: Admin privileges required".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            Err((StatusCode::FORBIDDEN, Json(error_response)).into_response())
        }
    }
}

pub fn extract_auth_context(request: &Request) -> Result<&AuthContext, AppError> {
    request
        .extensions()
        .get::<AuthContext>()
        .ok_or_else(|| AppError::Authorization("Authentication required".to_string()))
}

/// Helper function to validate parent access to their own data
pub fn validate_parent_access(auth: &AuthContext, requested_parent_id: &Uuid) -> Result<(), AppError> {
    match auth.role {
        UserRole::Parent => {
            // Parents can only access their own data
            if auth.user_id != *requested_parent_id {
                return Err(AppError::Authorization(
                    "Access denied: You can only access your own data".to_string()
                ));
            }
            Ok(())
        }
        UserRole::Admin | UserRole::SuperAdmin => {
            // Admins can access any parent's data
            Ok(())
        }
        _ => {
            // Other roles (Teacher, etc.) cannot access parent data
            Err(AppError::Authorization(
                "Access denied: Insufficient permissions".to_string()
            ))
        }
    }
}

pub fn check_permission_admin_or_superadmin(auth: &AuthContext, school_id: &Uuid) -> Result<(), AppError> {
    match auth.role {
        UserRole::SuperAdmin => Ok(()),
        UserRole::Admin if &auth.school_id == school_id => Ok(()),
        _ => Err(AppError::Authorization("Insufficient permissions".to_string())),
    }
}

pub fn check_permission_school_access(auth: &AuthContext, school_id: &Uuid) -> Result<(), AppError> {
    match auth.role {
        UserRole::SuperAdmin => Ok(()),
        _ if &auth.school_id == school_id => Ok(()),
        _ => Err(AppError::Authorization("Access denied to school".to_string())),
    }
}

pub fn check_permission_superadmin_only(auth: &AuthContext) -> Result<(), AppError> {
    match auth.role {
        UserRole::SuperAdmin => Ok(()),
        _ => Err(AppError::Authorization("SuperAdmin access required".to_string())),
    }
}

#[derive(Debug, Clone)]
pub struct SchoolContext {
    pub school_id: Uuid,
}

pub async fn school_header_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try to get school_id from X-School-ID header first
    let school_id = if let Some(school_header) = headers.get("X-School-ID") {
        // Parse school_id from header
        let school_id_str = school_header.to_str()
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        Uuid::parse_str(school_id_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
    } else {
        // Fallback to AuthContext school_id from JWT
        let auth_context = request.extensions()
            .get::<AuthContext>()
            .ok_or(StatusCode::UNAUTHORIZED)?;

        auth_context.school_id
    };

    let school_context = SchoolContext { school_id };
    request.extensions_mut().insert(school_context);

    Ok(next.run(request).await)
}

pub fn extract_school_context(request: &Request) -> Result<&SchoolContext, AppError> {
    request
        .extensions()
        .get::<SchoolContext>()
        .ok_or_else(|| AppError::Authorization("School context required".to_string()))
}

// Simplified permission checks that use school context
pub fn check_admin_or_superadmin_permission(auth: &AuthContext, school_context: &SchoolContext) -> Result<(), AppError> {
    match auth.role {
        UserRole::SuperAdmin => Ok(()),
        UserRole::Admin if auth.school_id == school_context.school_id => Ok(()),
        _ => Err(AppError::Authorization("Insufficient permissions".to_string())),
    }
}

pub fn check_school_access_permission(auth: &AuthContext, school_context: &SchoolContext) -> Result<(), AppError> {
    match auth.role {
        UserRole::SuperAdmin => Ok(()),
        _ if auth.school_id == school_context.school_id => Ok(()),
        _ => Err(AppError::Authorization("Access denied to school".to_string())),
    }
}

/// Helper function to verify JWT using Supabase API
async fn verify_jwt_with_supabase(jwt_token: &str) -> Result<AuthContext, AppError> {
    let supabase_url = env::var("SUPABASE_URL")
        .map_err(|_| AppError::Internal("SUPABASE_URL not configured".to_string()))?;
    let anon_key = env::var("SUPABASE_ANON_KEY")
        .map_err(|_| AppError::Internal("SUPABASE_ANON_KEY not configured".to_string()))?;

    let client = Client::new();
    let url = format!("{}/auth/v1/user", supabase_url);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("apikey", &anon_key)
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("Failed to verify JWT: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AppError::Authorization(format!("Invalid JWT token: {}", error_text)));
    }

    let user_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::ExternalService(format!("Failed to parse user data: {}", e)))?;

    println!("[DEBUG] Supabase user data: {:?}", user_data);

    // Extract user ID from response
    let user_id_str = user_data.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Authorization("Missing user ID in JWT response".to_string()))?;

    let user_id = Uuid::parse_str(user_id_str)
        .map_err(|_| AppError::Authorization("Invalid user ID format".to_string()))?;

    // Extract email
    let email = user_data.get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Authorization("Missing email in JWT response".to_string()))?
        .to_string();

    // Extract role and school_id from user_metadata
    let user_metadata = user_data.get("user_metadata")
        .ok_or_else(|| AppError::Authorization("Missing user_metadata in JWT response".to_string()))?;

    let role_str = user_metadata.get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Authorization("Missing role in user_metadata".to_string()))?;

    let role = match role_str {
        "Admin" => UserRole::Admin,
        "SuperAdmin" => UserRole::SuperAdmin,
        "Parent" => UserRole::Parent,
        "Teacher" => UserRole::Teacher,
        _ => {
            println!("[WARN] Unknown role in JWT: {}", role_str);
            UserRole::Parent // Default to Parent for safety
        }
    };

    let school_id_str = user_metadata.get("school_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Authorization("Missing school_id in user_metadata".to_string()))?;

    let school_id = Uuid::parse_str(school_id_str)
        .map_err(|_| AppError::Authorization("Invalid school_id format".to_string()))?;

    Ok(AuthContext {
        user_id,
        school_id,
        role,
        email,
    })
}

/// Combined JWT or API Key middleware - allows either authentication method
/// First tries JWT authentication, falls back to API key if JWT is not present or invalid
pub async fn jwt_or_api_key_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // First, try JWT authentication
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            // Try JWT authentication using Supabase API
            let token = &auth_header[7..];

            match verify_jwt_with_supabase(token).await {
                Ok(auth_context) => {
                    println!("[DEBUG] Supabase API JWT Auth Success - User: {}, Role: {:?}, School: {}",
                        auth_context.email, auth_context.role, auth_context.school_id);
                    request.extensions_mut().insert(auth_context);
                    return Ok(next.run(request).await);
                }
                Err(e) => {
                    println!("[DEBUG] Supabase API JWT validation failed: {:?}", e);
                    // JWT validation failed, will try API key fallback
                }
            }
        }
    }

    // JWT not present or invalid, try API key authentication
    println!("[DEBUG] JWT authentication failed or not present, trying API key fallback");
    api_key_fallback(headers, request, next).await
}

/// Combined JWT (admin-only) or API Key middleware
/// For admin-only endpoints that can also be accessed with API key for testing
pub async fn jwt_or_api_key_admin_only(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // First, try JWT authentication
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            // Try JWT authentication using Supabase API
            let token = &auth_header[7..];

            match verify_jwt_with_supabase(token).await {
                Ok(auth_context) => {
                    // Check if user has admin privileges
                    match auth_context.role {
                        UserRole::Admin | UserRole::SuperAdmin => {
                            println!("[DEBUG] Supabase API JWT Admin Auth Success - User: {}, Role: {:?}",
                                auth_context.email, auth_context.role);
                            request.extensions_mut().insert(auth_context);
                            return Ok(next.run(request).await);
                        }
                        _ => {
                            println!("[DEBUG] JWT Auth failed: User does not have admin role");
                            // User is authenticated but not authorized (not admin) - return 403 Forbidden
                            let error_response = ErrorResponse {
                                success: false,
                                message: "Access forbidden. Admin role required.".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };
                            return Err((
                                axum::http::StatusCode::FORBIDDEN,
                                Json(error_response),
                            ).into_response());
                        }
                    }
                }
                Err(e) => {
                    println!("[DEBUG] Supabase API JWT validation failed in admin check: {:?}", e);
                    // JWT validation failed, will try API key fallback
                }
            }
        }
    }

    // JWT not present, invalid, or not admin - try API key authentication
    println!("[DEBUG] JWT admin authentication failed or not present, trying API key fallback");
    api_key_fallback(headers, request, next).await
}

/// SuperAdmin-only middleware - allows ONLY SuperAdmin role
/// For endpoints that should be restricted to SuperAdmin only
pub async fn jwt_or_api_key_superadmin_only(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // First, try JWT authentication
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            // Try JWT authentication using Supabase API
            let token = &auth_header[7..];

            match verify_jwt_with_supabase(token).await {
                Ok(auth_context) => {
                    // Check if user has SuperAdmin role ONLY
                    match auth_context.role {
                        UserRole::SuperAdmin => {
                            println!("[DEBUG] Supabase API JWT SuperAdmin Auth Success - User: {}, Role: {:?}",
                                auth_context.email, auth_context.role);
                            request.extensions_mut().insert(auth_context);
                            return Ok(next.run(request).await);
                        }
                        _ => {
                            println!("[DEBUG] JWT Auth failed: User does not have SuperAdmin role");
                            // User is authenticated but not authorized (not SuperAdmin) - return 403 Forbidden
                            let error_response = ErrorResponse {
                                success: false,
                                message: "Access forbidden. SuperAdmin role required.".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };
                            return Err((
                                StatusCode::FORBIDDEN,
                                Json(error_response),
                            ).into_response());
                        }
                    }
                }
                Err(e) => {
                    println!("[DEBUG] Supabase API JWT validation failed in SuperAdmin check: {:?}", e);
                    // JWT validation failed, will try API key fallback
                }
            }
        }
    }

    // JWT not present, invalid, or not SuperAdmin - try API key authentication
    println!("[DEBUG] JWT SuperAdmin authentication failed or not present, trying API key fallback");
    api_key_fallback(headers, request, next).await
}

/// Helper function to handle API key authentication fallback
async fn api_key_fallback(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    println!("[DEBUG] api_key_fallback: Checking for X-API-Key header");

    let api_key = headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok());

    println!("[DEBUG] api_key_fallback: X-API-Key present: {}", api_key.is_some());

    if api_key.is_none() {
        println!("[DEBUG] api_key_fallback: No X-API-Key found, returning 401");
        let error_response = ErrorResponse {
            success: false,
            message: "Authentication required. Please provide either Bearer token or X-API-Key header".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    let owner_api_key = match env::var("OWNER_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            let error_response = ErrorResponse {
                success: false,
                message: "Server configuration error".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response());
        }
    };

    if api_key.unwrap() != owner_api_key {
        let error_response = ErrorResponse {
            success: false,
            message: "Invalid API key or JWT token".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response());
    }

    // API key is valid, create a mock auth context for API key access
    // This allows API key holders to act as SuperAdmin for testing
    let mock_auth_context = AuthContext {
        user_id: Uuid::nil(), // Use nil UUID for API key access
        school_id: Uuid::nil(), // Will need to be handled by services
        role: UserRole::SuperAdmin, // API key grants SuperAdmin access for testing
        email: "api-key-user@test.com".to_string(),
    };

    request.extensions_mut().insert(mock_auth_context);
    Ok(next.run(request).await)
}