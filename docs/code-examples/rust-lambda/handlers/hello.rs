use lambda_web::{Request, Result, Body, Response};
use serde_json::json;
use tracing::{info, error};
use crate::{ApiResponse, HealthData, HelloData, extract_path_params};

/// Hello World endpoint
pub async fn hello_world(request: Request) -> Result<Response<Body>> {
    info!("Hello World endpoint called");

    let data = HelloData {
        greeting: "Hello, World!".to_string(),
        name: None,
    };

    let response = ApiResponse::success(data, "Welcome to the Rust Lambda API!");

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("access-control-allow-headers", "Content-Type, Authorization")
        .body(serde_json::to_string(&response)?.into())?)
}

/// Health check endpoint
pub async fn health_check(request: Request) -> Result<Response<Body>> {
    info!("Health check endpoint called");

    let data = HealthData {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string(),
    };

    let response = ApiResponse::success(data, "Service is healthy");

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .body(serde_json::to_string(&response)?.into())?)
}

/// Personalized hello endpoint with path parameter
pub async fn hello_name(request: Request) -> Result<Response<Body>> {
    let path = request.uri().path();
    info!("Hello name endpoint called with path: {}", path);

    // Extract name from path parameters
    let params = extract_path_params(path, "/hello/{name}");
    let name = params.get("name").cloned().unwrap_or_else(|| "Unknown".to_string());

    let data = HelloData {
        greeting: format!("Hello, {}!", name),
        name: Some(name),
    };

    let response = ApiResponse::success(data, "Personalized greeting generated");

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("access-control-allow-headers", "Content-Type, Authorization")
        .body(serde_json::to_string(&response)?.into())?)
}

/// Handle OPTIONS requests for CORS
pub async fn options_handler(request: Request) -> Result<Response<Body>> {
    info!("OPTIONS request received");

    Ok(Response::builder()
        .status(204)
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("access-control-allow-headers", "Content-Type, Authorization")
        .header("access-control-max-age", "86400")
        .body("".into())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_web::http::{Method, Uri};

    fn create_test_request(uri: &str) -> Request {
        Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::Empty)
            .unwrap()
    }

    #[tokio::test]
    async fn test_hello_world() {
        let request = create_test_request("/");
        let response = hello_world(request).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_health_check() {
        let request = create_test_request("/health");
        let response = health_check(request).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_hello_name() {
        let request = create_test_request("/hello/TestUser");
        let response = hello_name(request).await.unwrap();
        assert_eq!(response.status(), 200);
    }
}