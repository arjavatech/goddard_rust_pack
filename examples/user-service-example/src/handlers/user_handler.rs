use serde_json::{json, Value};
use tracing::{info, error, warn};

use crate::models::{CreateUserRequest, UpdateUserRequest, UserResponse};
use crate::services::{UserService, UserServiceError};

#[derive(serde::Deserialize)]
struct ApiGatewayRequest {
    #[serde(rename = "httpMethod")]
    http_method: String,
    path: String,
    #[serde(rename = "pathParameters")]
    path_parameters: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
}

#[derive(serde::Serialize)]
struct ApiGatewayResponse {
    #[serde(rename = "statusCode")]
    status_code: u16,
    headers: std::collections::HashMap<String, String>,
    body: String,
}

impl ApiGatewayResponse {
    fn new(status_code: u16, body: Value) -> Self {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        headers.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, PUT, DELETE, OPTIONS".to_string());
        headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type, Authorization".to_string());

        Self {
            status_code,
            headers,
            body: body.to_string(),
        }
    }

    fn success(data: UserResponse) -> Self {
        Self::new(200, json!(data))
    }

    fn created(data: UserResponse) -> Self {
        Self::new(201, json!(data))
    }

    fn bad_request(message: &str) -> Self {
        Self::new(400, json!({
            "error": "Bad Request",
            "message": message,
            "success": false
        }))
    }

    fn not_found(message: &str) -> Self {
        Self::new(404, json!({
            "error": "Not Found", 
            "message": message,
            "success": false
        }))
    }

    fn internal_error(message: &str) -> Self {
        Self::new(500, json!({
            "error": "Internal Server Error",
            "message": message,
            "success": false
        }))
    }
}

pub async fn handle_request(body: String, request_id: &str) -> Result<crate::Response, String> {
    info!("Handling request: {}", request_id);

    // Parse the API Gateway event
    let api_request: ApiGatewayRequest = serde_json::from_str(&body)
        .map_err(|e| {
            error!("Failed to parse API Gateway request: {}", e);
            format!("Invalid request format: {}", e)
        })?;

    info!("Method: {}, Path: {}", api_request.http_method, api_request.path);

    // Initialize user service
    let user_service = UserService::new().await
        .map_err(|e| {
            error!("Failed to initialize user service: {}", e);
            format!("Service initialization error: {}", e)
        })?;

    // Route the request
    let response = match route_request(api_request, &user_service).await {
        Ok(api_response) => {
            info!("Request processed successfully");
            crate::Response {
                req_id: request_id.to_string(),
                msg: "Request processed".to_string(),
                status_code: api_response.status_code,
            }
        },
        Err(e) => {
            error!("Request processing failed: {}", e);
            crate::Response {
                req_id: request_id.to_string(),
                msg: format!("Error: {}", e),
                status_code: 500,
            }
        }
    };

    Ok(response)
}

async fn route_request(
    request: ApiGatewayRequest,
    user_service: &UserService,
) -> Result<ApiGatewayResponse, String> {
    match (request.http_method.as_str(), request.path.as_str()) {
        ("GET", "/users") => handle_list_users(user_service).await,
        ("POST", "/users") => {
            let body = request.body.ok_or("Missing request body")?;
            handle_create_user(body, user_service).await
        },
        ("GET", path) if path.starts_with("/users/") => {
            let id = path.strip_prefix("/users/").unwrap();
            handle_get_user(id, user_service).await
        },
        ("PUT", path) if path.starts_with("/users/") => {
            let id = path.strip_prefix("/users/").unwrap();
            let body = request.body.ok_or("Missing request body")?;
            handle_update_user(id, body, user_service).await
        },
        ("DELETE", path) if path.starts_with("/users/") => {
            let id = path.strip_prefix("/users/").unwrap();
            handle_delete_user(id, user_service).await
        },
        ("OPTIONS", _) => {
            // Handle CORS preflight
            Ok(ApiGatewayResponse::new(200, json!({"message": "OK"})))
        },
        _ => {
            warn!("Unhandled route: {} {}", request.http_method, request.path);
            Ok(ApiGatewayResponse::not_found("Route not found"))
        }
    }
}

async fn handle_create_user(
    body: String,
    user_service: &UserService,
) -> Result<ApiGatewayResponse, String> {
    info!("Creating user");

    let create_request: CreateUserRequest = serde_json::from_str(&body)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    match user_service.create_user(create_request).await {
        Ok(response) => Ok(ApiGatewayResponse::created(response)),
        Err(UserServiceError::Validation(msg)) => Ok(ApiGatewayResponse::bad_request(&msg)),
        Err(e) => {
            error!("Create user error: {}", e);
            Ok(ApiGatewayResponse::internal_error("Failed to create user"))
        }
    }
}

async fn handle_get_user(
    id: &str,
    user_service: &UserService,
) -> Result<ApiGatewayResponse, String> {
    info!("Getting user: {}", id);

    match user_service.get_user(id).await {
        Ok(response) => Ok(ApiGatewayResponse::success(response)),
        Err(UserServiceError::NotFound(_)) => {
            Ok(ApiGatewayResponse::not_found("User not found"))
        },
        Err(e) => {
            error!("Get user error: {}", e);
            Ok(ApiGatewayResponse::internal_error("Failed to get user"))
        }
    }
}

async fn handle_update_user(
    id: &str,
    body: String,
    user_service: &UserService,
) -> Result<ApiGatewayResponse, String> {
    info!("Updating user: {}", id);

    let update_request: UpdateUserRequest = serde_json::from_str(&body)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    match user_service.update_user(id, update_request).await {
        Ok(response) => Ok(ApiGatewayResponse::success(response)),
        Err(UserServiceError::NotFound(_)) => {
            Ok(ApiGatewayResponse::not_found("User not found"))
        },
        Err(UserServiceError::Validation(msg)) => Ok(ApiGatewayResponse::bad_request(&msg)),
        Err(e) => {
            error!("Update user error: {}", e);
            Ok(ApiGatewayResponse::internal_error("Failed to update user"))
        }
    }
}

async fn handle_delete_user(
    id: &str,
    user_service: &UserService,
) -> Result<ApiGatewayResponse, String> {
    info!("Deleting user: {}", id);

    match user_service.delete_user(id).await {
        Ok(response) => Ok(ApiGatewayResponse::success(response)),
        Err(UserServiceError::NotFound(_)) => {
            Ok(ApiGatewayResponse::not_found("User not found"))
        },
        Err(e) => {
            error!("Delete user error: {}", e);
            Ok(ApiGatewayResponse::internal_error("Failed to delete user"))
        }
    }
}

async fn handle_list_users(
    user_service: &UserService,
) -> Result<ApiGatewayResponse, String> {
    info!("Listing users");

    match user_service.list_users(Some(100)).await {
        Ok(response) => Ok(ApiGatewayResponse::success(response)),
        Err(e) => {
            error!("List users error: {}", e);
            Ok(ApiGatewayResponse::internal_error("Failed to list users"))
        }
    }
}