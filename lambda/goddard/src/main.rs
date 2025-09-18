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
    form_template_controller::{
        create_form_template, get_form_templates_by_school, update_form_template, delete_form_template
    },
    class_form_override_controller::{
        create_class_form_override, delete_class_form_override
    },
};
use middleware::{request_id::request_id_middleware, cors::add_cors_headers};
use config::database::{initialize_database, get_db_pool};
use dao::{
    AuthDao, SchoolDao, ClassroomDao, FormTemplateDao, ClassFormOverrideDao, /* UserDao, EnrollmentDao */
};
use services::{
    AuthService, SupabaseClient, SchoolService, ClassroomService, FormTemplateService, ClassFormOverrideService, /* UserService, EnrollmentService */
};
use middleware::auth::{api_key_middleware};
use std::sync::Arc;




#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Check if we're running in Lambda environment
    let is_lambda = std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok();

    if is_lambda {
        // Configure lambda_http to ignore stage in path (e.g., /prod/health -> /health)
        std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
        if let Err(e) = run_lambda().await {
            return Err(format!("Lambda error: {}", e).into());
        }
    } else {
        run_local_server().await?;
    }
    Ok(())
}

async fn run_lambda() -> Result<(), lambda_http::Error> {
    // Initialize database connection
    initialize_database().await
        .map_err(|e| lambda_http::Error::from(format!("Database connection error: {}", e)))?;
    let pool = get_db_pool();

    // Initialize DAOs
    let auth_dao = AuthDao::new(pool.clone());
    let school_dao = SchoolDao::new(pool.clone());
    let classroom_dao = ClassroomDao::new(pool.clone());
    let form_template_dao = FormTemplateDao::new(pool.clone());
    let class_form_override_dao = ClassFormOverrideDao::new(pool.clone());

    // Initialize Supabase client
    let supabase_client = SupabaseClient::new()
        .map_err(|e| lambda_http::Error::from(format!("Supabase client error: {}", e)))?;

    // Initialize services
    let auth_service = Arc::new(AuthService::new(auth_dao, supabase_client.clone()));
    let school_service = Arc::new(SchoolService::new(school_dao));
    let classroom_service = Arc::new(ClassroomService::new(classroom_dao));
    let form_template_service = Arc::new(FormTemplateService::new(form_template_dao));
    let class_form_override_service = Arc::new(ClassFormOverrideService::new(class_form_override_dao));
    
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

        // Form Templates Management APIs (API Key Protected for Testing)
        .route("/form-templates", post(create_form_template).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/form-templates", get(get_form_templates_by_school).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/form-templates", put(update_form_template).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/form-templates", delete(delete_form_template).layer(axum_middleware::from_fn(api_key_middleware)))
        .with_state(form_template_service)

        // Class Form Overrides Management APIs (API Key Protected for Testing)
        .route("/class-form-overrides", post(create_class_form_override).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/class-form-overrides", delete(delete_class_form_override).layer(axum_middleware::from_fn(api_key_middleware)))
        .with_state(class_form_override_service)

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    run(app).await
}

async fn run_local_server() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database connection
    initialize_database().await?;
    let pool = get_db_pool();

    // Initialize DAOs
    let auth_dao = AuthDao::new(pool.clone());
    let school_dao = SchoolDao::new(pool.clone());
    let classroom_dao = ClassroomDao::new(pool.clone());
    let form_template_dao = FormTemplateDao::new(pool.clone());
    let class_form_override_dao = ClassFormOverrideDao::new(pool.clone());

    // Initialize Supabase client
    let supabase_client = SupabaseClient::new()?;

    // Initialize services
    let auth_service = Arc::new(AuthService::new(auth_dao, supabase_client.clone()));
    let school_service = Arc::new(SchoolService::new(school_dao));
    let classroom_service = Arc::new(ClassroomService::new(classroom_dao));
    let form_template_service = Arc::new(FormTemplateService::new(form_template_dao));
    let class_form_override_service = Arc::new(ClassFormOverrideService::new(class_form_override_dao));

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
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

        // Form Templates Management APIs (API Key Protected for Testing)
        .route("/form-templates", post(create_form_template).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/form-templates", get(get_form_templates_by_school).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/form-templates", put(update_form_template).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/form-templates", delete(delete_form_template).layer(axum_middleware::from_fn(api_key_middleware)))
        .with_state(form_template_service)

        // Class Form Overrides Management APIs (API Key Protected for Testing)
        .route("/class-form-overrides", post(create_class_form_override).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/class-form-overrides", delete(delete_class_form_override).layer(axum_middleware::from_fn(api_key_middleware)))
        .with_state(class_form_override_service)

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    println!("Starting local server on http://localhost:8080");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Tests removed - will add proper integration tests later
