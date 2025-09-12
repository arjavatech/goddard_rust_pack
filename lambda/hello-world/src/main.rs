use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use lambda_web::{is_running_on_lambda, run_hyper_on_lambda, LambdaError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Serialize, Deserialize)]
struct HelloResponse {
    message: String,
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
}

async fn root_handler() -> Json<HelloResponse> {
    info!("Handling root request");
    Json(HelloResponse {
        message: "Hello from Rust Lambda!".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

async fn hello_handler(Path(name): Path<String>) -> Json<HelloResponse> {
    info!("Handling hello request for: {}", name);
    Json(HelloResponse {
        message: format!("Hello, {}! Welcome to Rust Lambda", name),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

async fn health_handler() -> Json<HealthResponse> {
    info!("Health check requested");
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "hello-world-lambda".to_string(),
    })
}

fn create_app() -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/hello/:name", get(hello_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    let app = create_app();

    if is_running_on_lambda() {
        info!("Running on AWS Lambda");
        run_hyper_on_lambda(app).await
    } else {
        info!("Running locally on port 3000");
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("Server listening on http://{}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_root_endpoint() {
        let app = create_app();
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/").await;
        response.assert_status(StatusCode::OK);
        
        let json: HelloResponse = response.json();
        assert!(json.message.contains("Hello from Rust Lambda"));
    }

    #[tokio::test]
    async fn test_hello_endpoint() {
        let app = create_app();
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/hello/World").await;
        response.assert_status(StatusCode::OK);
        
        let json: HelloResponse = response.json();
        assert_eq!(json.message, "Hello, World! Welcome to Rust Lambda");
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_app();
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
        
        let json: HealthResponse = response.json();
        assert_eq!(json.status, "healthy");
    }
}