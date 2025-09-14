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

use controllers::{
    hello_controller::{hello_world, health_check, hello_name},
    school_controller::{get_school, get_school_by_id, get_all_schools, create_school},
    users_controller::{list_users, create_user, get_user},
    children_controller::{list_children, create_child},
    classrooms_controller::{list_classrooms, create_classroom},
    enrollments_controller::{
        list_enrollments, create_enrollment, get_enrollment,
        update_enrollment, approve_enrollment, reject_enrollment
    },
    forms_controller::{list_form_templates, create_form_template, handle_fillout_webhook},
    notifications_controller::{list_additional_emails, create_additional_email},
    documents_controller::{list_documents, upload_document},
    admin_controller::get_dashboard_overview,
    api_discovery_controller::{get_all_endpoints, get_endpoints_by_category},
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

        // API Discovery
        .route("/api/endpoints", get(get_all_endpoints))
        .route("/api/endpoints/grouped", get(get_endpoints_by_category))

        .route("/school/:school_id", get(get_school_by_id))
        .route("/school", get(get_all_schools).post(create_school))

        // User Routes
        .route("/users", get(list_users).post(create_user))
        .route("/users/:user_id", get(get_user))

        // Children Routes
        .route("/children", get(list_children).post(create_child))

        // Classroom Routes
        .route("/classrooms", get(list_classrooms).post(create_classroom))

        // Enrollment Routes
        .route("/enrollments", get(list_enrollments).post(create_enrollment))
        .route("/enrollments/:enrollment_id", get(get_enrollment).patch(update_enrollment))
        .route("/enrollments/:enrollment_id/approve", post(approve_enrollment))
        .route("/enrollments/:enrollment_id/reject", post(reject_enrollment))

        // Form Routes
        .route("/form-templates", get(list_form_templates).post(create_form_template))
        .route("/form-submissions/webhook", post(handle_fillout_webhook))

        // Communication Routes
        .route("/notifications/emails", get(list_additional_emails).post(create_additional_email))

        // Document Routes
        .route("/documents", get(list_documents).post(upload_document))

        // Admin Routes
        .route("/admin/dashboard", get(get_dashboard_overview))

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    run(app).await
}

// Tests removed - will add proper integration tests later
