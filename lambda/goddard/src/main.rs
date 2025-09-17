use axum::{
    http::Method,
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use lambda_http::{run, Error};
use tower_http::cors::{Any, CorsLayer};

mod controllers;
mod middleware;
mod models;
mod db;
mod error;
mod utils;
mod config;
mod dao;
mod services;

use controllers::{
    hello_controller::{hello_world, health_check, hello_name},
    auth_verification_controller::{
        get_auth_verification_status,
        get_invitation_summary,
        resend_invitation,
        create_invitation
    },
};
use middleware::{request_id::request_id_middleware, cors::add_cors_headers};
use config::DatabaseConfig;
use dao::AuthDao;
use services::{AuthService, SupabaseClient};
use std::sync::Arc;




#[tokio::main]
async fn main() -> Result<(), Error> {
    // Configure lambda_http to ignore stage in path (e.g., /prod/health -> /health)
    std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize database connection
    let db_config = DatabaseConfig::from_env();
    let pool = db_config.create_pool().await
        .map_err(|e| lambda_http::Error::from(format!("Database connection error: {}", e)))?;

    // Initialize services
    let auth_dao = AuthDao::new(pool);
    let supabase_client = SupabaseClient::new()
        .map_err(|e| lambda_http::Error::from(format!("Supabase client error: {}", e)))?;
    let auth_service = Arc::new(AuthService::new(auth_dao, supabase_client));
    
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

        // Authorization Verification Routes
        .route("/auth/verification-status", get(get_auth_verification_status))
        .route("/auth/invitation-summary", get(get_invitation_summary))
        .route("/auth/resend-invitation", post(resend_invitation))
        .route("/auth/invite-create", post(create_invitation))

        // Add service dependencies
        .with_state(auth_service)

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    run(app).await
}

// Tests removed - will add proper integration tests later
