use lambda_http::{run, service_fn, Error, Request, Response, Body};
use lambda_runtime::tracing;
use serde::{Deserialize, Serialize};

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

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    tracing::info!("Processing request: {}", event.uri());
    
    let path = event.uri().path();
    let method = event.method().as_str();
    
    tracing::info!("Method: {}, Path: {}", method, path);
    
    match (method, path) {
        ("GET", "/") => {
            let response = HelloResponse {
                message: "Hello from Rust Lambda!".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                path: path.to_string(),
            };
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .header("access-control-allow-origin", "*")
                .header("access-control-allow-methods", "GET, POST, OPTIONS")
                .header("access-control-allow-headers", "Content-Type")
                .body(serde_json::to_string(&response)?.into())?)
        },
        ("GET", "/health") => {
            let response = HealthResponse {
                status: "healthy".to_string(),
                service: "hello-world-lambda".to_string(),
            };
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .header("access-control-allow-origin", "*")
                .body(serde_json::to_string(&response)?.into())?)
        },
        ("GET", path) if path.starts_with("/hello/") => {
            let name = path.strip_prefix("/hello/").unwrap_or("World");
            let response = HelloResponse {
                message: format!("Hello, {}! Welcome to Rust Lambda", name),
                timestamp: chrono::Utc::now().to_rfc3339(),
                path: path.to_string(),
            };
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .header("access-control-allow-origin", "*")
                .body(serde_json::to_string(&response)?.into())?)
        },
        ("OPTIONS", _) => {
            // Handle CORS preflight requests
            Ok(Response::builder()
                .status(200)
                .header("access-control-allow-origin", "*")
                .header("access-control-allow-methods", "GET, POST, OPTIONS")
                .header("access-control-allow-headers", "Content-Type")
                .body("".into())?)
        },
        _ => {
            let error_response = serde_json::json!({
                "error": "Not Found",
                "message": format!("Path {} not found", path),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            Ok(Response::builder()
                .status(404)
                .header("content-type", "application/json")
                .header("access-control-allow-origin", "*")
                .body(error_response.to_string().into())?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
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