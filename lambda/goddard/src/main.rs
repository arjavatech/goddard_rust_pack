use axum::{
    http::Method,
    middleware as axum_middleware,
    routing::get,
    Router,
};
use lambda_http::{run, Error};
use tower_http::cors::{Any, CorsLayer};

mod controllers;
mod middleware;
mod models;
mod db;

use controllers::{
    hello_controller::{hello_world, health_check, hello_name},
};
use middleware::{request_id::request_id_middleware, cors::add_cors_headers};




#[tokio::main]
async fn main() -> Result<(), Error> {
    // Configure lambda_http to ignore stage in path (e.g., /prod/health -> /health)
    std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // Load environment variables
    dotenv::dotenv().ok();
    
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
        // Health and Info Routes
        .route("/", get(hello_world))
        .route("/health", get(health_check))
        .route("/hello/:name", get(hello_name))

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    run(app).await
}

// Tests removed - will add proper integration tests later
