use axum::{
    extract::Path,
    http::{HeaderValue, Method, StatusCode},
    middleware::{self},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use lambda_http::{run, Error};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize, Deserialize)]
struct HelloResponse {
    message: String,
    timestamp: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
}

async fn hello_world() -> impl IntoResponse {
    tracing::info!("Hello world endpoint called!");
    let response = HelloResponse {
        message: "Hello from Rust Lambda!".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        path: "/".to_string(),
    };
    (StatusCode::OK, axum::Json(response))
}

async fn health_check() -> impl IntoResponse {
    tracing::info!("Health check endpoint called!");
    let response = HealthResponse {
        status: "healthy".to_string(),
        service: "hello-world-lambda".to_string(),
    };
    (StatusCode::OK, axum::Json(response))
}

async fn hello_name(Path(name): Path<String>) -> impl IntoResponse {
    tracing::info!("Hello name endpoint called with name: {}", name);
    let response = HelloResponse {
        message: format!("Hello, {}! Welcome to Rust Lambda", name),
        timestamp: chrono::Utc::now().to_rfc3339(),
        path: format!("/hello/{}", name),
    };
    (StatusCode::OK, axum::Json(response))
}


// Middleware to add request ID
async fn request_id_middleware(
    mut req: axum::extract::Request,
    next: middleware::Next,
) -> impl IntoResponse {
    let request_id = uuid::Uuid::new_v4().to_string();
    tracing::info!("Request ID: {}", request_id);
    
    // Add request ID to request extensions
    req.extensions_mut().insert(request_id.clone());
    
    // Process the request
    let mut response = next.run(req).await;
    
    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("X-Request-ID", header_value);
    }
    
    response
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Configure lambda_http to ignore stage in path (e.g., /prod/health -> /health)
    std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(vec![
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/health", get(health_check))
        .route("/hello/:name", get(hello_name))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(cors);

    run(app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::http::{Method, Request as HttpRequest};

    fn create_test_request(method: Method, uri: &str) -> Request {
        let http_request = HttpRequest::builder()
            .method(method)
            .uri(uri)
            .body(Body::Empty)
            .unwrap();
        
        Request::from(http_request)
    }

    #[tokio::test]
    async fn test_root_endpoint() {
        let request = create_test_request(Method::GET, "/");
        let response = function_handler(request).await.unwrap();
        
        assert_eq!(response.status(), 200);
        
        let body = match response.body() {
            Body::Text(text) => text,
            _ => panic!("Expected text body"),
        };
        
        let json: HelloResponse = serde_json::from_str(body).unwrap();
        assert!(json.message.contains("Hello from Rust Lambda"));
        assert_eq!(json.path, "/");
    }

    #[tokio::test]
    async fn test_hello_endpoint() {
        let request = create_test_request(Method::GET, "/hello/World");
        let response = function_handler(request).await.unwrap();
        
        assert_eq!(response.status(), 200);
        
        let body = match response.body() {
            Body::Text(text) => text,
            _ => panic!("Expected text body"),
        };
        
        let json: HelloResponse = serde_json::from_str(body).unwrap();
        assert_eq!(json.message, "Hello, World! Welcome to Rust Lambda");
        assert_eq!(json.path, "/hello/World");
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let request = create_test_request(Method::GET, "/health");
        let response = function_handler(request).await.unwrap();
        
        assert_eq!(response.status(), 200);
        
        let body = match response.body() {
            Body::Text(text) => text,
            _ => panic!("Expected text body"),
        };
        
        let json: HealthResponse = serde_json::from_str(body).unwrap();
        assert_eq!(json.status, "healthy");
    }

    #[tokio::test]
    async fn test_not_found() {
        let request = create_test_request(Method::GET, "/nonexistent");
        let response = function_handler(request).await.unwrap();
        
        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn test_cors_preflight() {
        let request = create_test_request(Method::OPTIONS, "/");
        let response = function_handler(request).await.unwrap();
        
        assert_eq!(response.status(), 200);
        assert!(response.headers().contains_key("access-control-allow-origin"));
    }
}