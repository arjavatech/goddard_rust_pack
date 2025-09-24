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
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    let jwt_secret = env::var("JWT_SECRET")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["goddard-school"]);
    validation.set_issuer(&["goddard-auth"]);

    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let auth_context = AuthContext {
        user_id: token_data.claims.user_id,
        school_id: token_data.claims.school_id,
        role: token_data.claims.role,
        email: token_data.claims.email,
    };

    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}

pub fn extract_auth_context(request: &Request) -> Result<&AuthContext, AppError> {
    request
        .extensions()
        .get::<AuthContext>()
        .ok_or_else(|| AppError::Authorization("Authentication required".to_string()))
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