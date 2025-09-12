use lambda_web::{is_running_on_lambda, launch, LambdaError};
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;

mod handlers;
use handlers::hello;

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    info!("Starting Rust Lambda function");

    // Define routes
    let routes = vec![
        ("/", "GET", hello::hello_world),
        ("/health", "GET", hello::health_check),
        ("/hello/{name}", "GET", hello::hello_name),
    ];

    // Launch the Lambda web server
    launch(routes).await
}