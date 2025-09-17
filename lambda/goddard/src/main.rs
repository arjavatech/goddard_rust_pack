use axum::{
    http::Method,
    middleware as axum_middleware,
    routing::{get, post, put, delete},
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
        create_invitation
    },
    school_controller::{
        create_school, get_all_schools, update_school, delete_school
    },
    classroom_controller::{
        create_classroom, get_classrooms_by_school, update_classroom, delete_classroom
    },
};
use middleware::{request_id::request_id_middleware, cors::add_cors_headers};
use config::DatabaseConfig;
use dao::{
    AuthDao, SchoolDao, ClassroomDao, /* UserDao, FormTemplateDao, EnrollmentDao */
};
use services::{
    AuthService, SupabaseClient, SchoolService, ClassroomService, /* UserService, FormTemplateService, EnrollmentService */
};
use middleware::auth::{api_key_middleware};
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

    // Initialize DAOs
    let auth_dao = AuthDao::new(pool.clone());
    let school_dao = SchoolDao::new(pool.clone());
    let classroom_dao = ClassroomDao::new(pool.clone());

    // Initialize Supabase client
    let supabase_client = SupabaseClient::new()
        .map_err(|e| lambda_http::Error::from(format!("Supabase client error: {}", e)))?;

    // Initialize services
    let auth_service = Arc::new(AuthService::new(auth_dao, supabase_client.clone()));
    let school_service = Arc::new(SchoolService::new(school_dao));
    let classroom_service = Arc::new(ClassroomService::new(classroom_dao));
    
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

        // Authorization Verification Routes (Legacy)
        .route("/auth/verification-status", get(get_auth_verification_status))
        .route("/auth/invitation-summary", get(get_invitation_summary))
        // .route("/auth/resend-invitation", post(resend_invitation)) // DISABLED - resend_invitation not available
        .route("/auth/invite-create", post(create_invitation))
        .with_state(auth_service)

        // School Management APIs (API Key Protected)
        .route("/schools", post(create_school).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/schools", get(get_all_schools)) // Public
        .route("/schools", put(update_school).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/schools/:id", delete(delete_school).layer(axum_middleware::from_fn(api_key_middleware)))
        .with_state(school_service)

        // Classroom Management APIs (API Key Protected for Testing)
        .route("/classrooms", post(create_classroom).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/classrooms", get(get_classrooms_by_school).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/classrooms", put(update_classroom).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/classrooms", delete(delete_classroom).layer(axum_middleware::from_fn(api_key_middleware)))
        .with_state(classroom_service)

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    run(app).await
}

// Tests removed - will add proper integration tests later
