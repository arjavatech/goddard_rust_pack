use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use lambda_web::{is_running_on_lambda, launch, LambdaError};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, error, warn};

mod handlers;
mod models;
mod services;

use handlers::user_handler;

#[derive(Deserialize)]
struct Request {
    body: Option<String>,
}

#[derive(Serialize)]
struct Response {
    req_id: String,
    msg: String,
    status_code: u16,
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let (event, context) = event.into_parts();
    
    info!("Processing request with ID: {}", context.request_id);
    
    // Extract HTTP method and path from event
    // In a real implementation, you'd parse the API Gateway event properly
    let response = match event.body {
        Some(body) => {
            info!("Request body: {}", body);
            user_handler::handle_request(body, &context.request_id).await
                .map_err(|e| {
                    error!("Handler error: {}", e);
                    Error::from(format!("Handler error: {}", e))
                })?
        },
        None => {
            warn!("No body in request");
            Response {
                req_id: context.request_id.clone(),
                msg: "No body provided".to_string(),
                status_code: 400,
            }
        }
    };

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
        )
        .init();

    info!("Starting user service lambda");

    if is_running_on_lambda() {
        info!("Running on AWS Lambda");
        run(service_fn(function_handler)).await
    } else {
        info!("Running locally");
        // For local development, you can use lambda_web for HTTP server
        launch(function_handler).await.map_err(Error::from)
    }
}