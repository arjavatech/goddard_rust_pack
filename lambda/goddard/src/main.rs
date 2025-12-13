use axum::{
    http::Method,
    middleware as axum_middleware,
    routing::{get, post, put, delete, patch},
    Router,
};
use lambda_http::run;
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
        create_invitation,
        create_invitation_enhanced,
        create_superadmin,
        clear_auth_table,
        debug_auth_users,
        get_users_by_school_and_role,
        get_current_user_profile,
        get_admins_by_school,
        update_admin_user,
        delete_admin_user
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
    enrollment_controller::{
        create_parent_invite, resend_parent_confirmation, add_child, get_parent_details_by_school, get_enrollment_children_with_forms, get_school_forms, get_class_wise_count, get_class_based_enrollments, deactivate_parent, activate_parent, update_child_status, promote_enrollment, edit_class_transition
    },
    parent_details_controller::{
        get_parent_details_by_id
    },
    form_submission_controller::{
        create_form_submission_webhook, get_latest_form_submission, get_form_submission_versions,
        get_form_submission_by_id, update_form_submission_status
    },
    student_form_assignment_controller::{
        create_student_form_assignment, get_assignments_by_school, update_student_form_assignment, delete_student_form_assignment, bulk_assign_forms_to_students, assign_form_to_school_students
    },
    student_form_assignment_review_controller::{
        review_student_form_assignment
    },
    portal_controller::{
        get_user_context, get_parent_children, get_child_profile, get_child_forms,
        get_classroom_details, get_classroom_forms, assign_classroom_form, remove_classroom_form,
        get_parent_profile, get_child_demographics
    },
    admin_controller::{
        get_admin_dashboard_metrics
    },
};
use middleware::{request_id::request_id_middleware, cors::add_cors_headers};
use config::database::{initialize_database, get_db_pool};
use dao::{
    AuthDao, SchoolDao, ClassroomDao, FormTemplateDao, ClassFormOverrideDao, EnrollmentDao, FormSubmissionDao, StudentFormAssignmentDao, PortalDao, AdminDao
};
use services::{
    AuthService, SupabaseClient, SchoolService, ClassroomService, FormTemplateService, ClassFormOverrideService, EnrollmentService, FormSubmissionService, StudentFormAssignmentService, PortalService, FilloutService, AdminService
};
use middleware::auth::{api_key_middleware, jwt_or_api_key_middleware, jwt_or_api_key_admin_only, jwt_or_api_key_superadmin_only};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize database connection
    initialize_database().await?;

    // Create the application router
    let app = create_app().await?;

    // Check if we're running in Lambda environment
    let is_lambda = std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok();

    if is_lambda {
        // Configure lambda_http to ignore stage in path (e.g., /prod/health -> /health)
        std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
        println!("Running in AWS Lambda mode");
        run(app).await.map_err(|e| format!("Lambda error: {}", e))?;
    } else {
        // Local/Docker mode
        let port = std::env::var("PORT").unwrap_or_else(|_| "9000".to_string());
        let addr = format!("0.0.0.0:{}", port);
        println!("Starting server on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

async fn create_app() -> Result<Router, Box<dyn std::error::Error>> {
    let pool = get_db_pool();

    // Initialize DAOs
    let auth_dao = AuthDao::new(pool.clone());
    let school_dao = SchoolDao::new(pool.clone());
    let classroom_dao = ClassroomDao::new(pool.clone());
    let form_template_dao = FormTemplateDao::new(pool.clone());
    let class_form_override_dao = ClassFormOverrideDao::new(pool.clone());
    let enrollment_dao = EnrollmentDao::new(pool.clone());
    let form_submission_dao = FormSubmissionDao::new(pool.clone());
    let student_form_assignment_dao = StudentFormAssignmentDao::new(pool.clone());
    let portal_dao = PortalDao::new(pool.clone());
    let admin_dao = AdminDao::new(pool.clone());

    // Initialize Supabase client
    let supabase_client = SupabaseClient::new()?;

    // Initialize Fillout service (optional - only if environment variables are present)
    let fillout_service = std::env::var("FILLOUT_API_KEY")
        .map(|api_key| {
            let base_url = std::env::var("FILLOUT_API_BASE_URL").ok();
            FilloutService::new(api_key, base_url)
        })
        .ok();

    if fillout_service.is_some() {
        println!("[DEBUG] Fillout service initialized successfully");
    } else {
        println!("[WARN] Fillout service not initialized - missing environment variables");
    }

    // Initialize services
    let auth_service = Arc::new(AuthService::new(auth_dao, supabase_client.clone()));
    let school_service = Arc::new(SchoolService::new(school_dao));
    let classroom_service = Arc::new(ClassroomService::new(classroom_dao));
    let form_template_service = Arc::new(FormTemplateService::new(form_template_dao));
    let class_form_override_service = Arc::new(ClassFormOverrideService::new(class_form_override_dao));
    let enrollment_service = Arc::new(EnrollmentService::new(enrollment_dao, supabase_client.clone()));
    let form_submission_service = Arc::new(
        if let Some(fillout) = fillout_service {
            FormSubmissionService::new_with_fillout(form_submission_dao, fillout)
        } else {
            FormSubmissionService::new(form_submission_dao)
        }
    );
    let student_form_assignment_service = Arc::new(StudentFormAssignmentService::new(student_form_assignment_dao));
    let portal_service = Arc::new(PortalService::new(Arc::new(portal_dao)));
    let admin_service = Arc::new(AdminService::new(admin_dao));

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(vec![
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderName::from_static("x-api-key"),
        ]);

    // Build the application router
    let app = Router::new()
        // Health and Info Routes
        .route("/", get(hello_world))
        .route("/health", get(health_check))
        .route("/hello/:name", get(hello_name))

        // Authorization Verification Routes (Legacy)
        .route("/auth/verification-status", get(get_auth_verification_status))
        .route("/auth/invitation-summary", get(get_invitation_summary))
        .route("/auth/invite-create", post(create_invitation).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        .route("/auth/invite-create-enhanced", post(create_invitation_enhanced).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        .route("/auth/create-superadmin", post(create_superadmin).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/auth/clear-table", delete(clear_auth_table))
        .route("/auth/debug-users", get(debug_auth_users))
        .route("/auth/users/filter", get(get_users_by_school_and_role))
        .route("/users/me", get(get_current_user_profile))
        .route("/users/admin",
            get(get_admins_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only))
            .merge(put(update_admin_user).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
            .merge(delete(delete_admin_user).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        )
        .with_state(auth_service)

        // School Management APIs (Admin JWT or API Key)
        .route("/schools", post(create_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/schools", get(get_all_schools)) // Public
        .route("/schools", put(update_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/schools/:id", delete(delete_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(school_service)

        // Classroom Management APIs (Admin JWT or API Key)
        .route("/classrooms", post(create_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms", get(get_classrooms_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms", put(update_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms", delete(delete_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(classroom_service)

        // Form Templates Management APIs (Admin JWT or API Key)
        .route("/form-templates", post(create_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-templates", get(get_form_templates_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-templates", put(update_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-templates", delete(delete_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(form_template_service)

        // Class Form Overrides Management APIs (Admin JWT or API Key)
        .route("/class-form-overrides", post(create_class_form_override).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/class-form-overrides", delete(delete_class_form_override).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(class_form_override_service)

        // Admin Dashboard APIs (JWT protected - Admin/SuperAdmin)
        .route("/admin/dashboard-metrics", get(get_admin_dashboard_metrics).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(admin_service)

        // Enrollment Management APIs (Admin JWT or API Key)
        .route("/enrollments/parent-invite", post(create_parent_invite).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/resend-confirmation", post(resend_parent_confirmation).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/add-child", post(add_child).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/parent-details-by-school", get(get_parent_details_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/parent/details", get(get_parent_details_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/children-forms", get(get_enrollment_children_with_forms).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments", get(get_school_forms).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/class-wise-count", get(get_class_wise_count).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/class-based-enrollments", get(get_class_based_enrollments).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/parent/:parent_id", get(get_parent_details_by_id).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/parent/:parent_id", delete(deactivate_parent).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/parent/:parent_id/activate", patch(activate_parent).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/children/:child_id/status", patch(update_child_status).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        // Class Transitions APIs (Admin JWT or API Key)
        .route("/class-promotions/:enrollment_id", post(promote_enrollment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/class-transitions/:transition_id", patch(edit_class_transition).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(enrollment_service)

        // Form Submissions Management APIs (Admin JWT or API Key)
        .route("/form-submissions/webhook", post(create_form_submission_webhook))
        .route("/form-submissions/latest", get(get_latest_form_submission).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-submissions/versions", get(get_form_submission_versions).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-submissions/:submission_id", get(get_form_submission_by_id).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-submissions/:submission_id/status", put(update_form_submission_status).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(form_submission_service)

        // Student Form Assignments Management APIs (Admin JWT or API Key)
        .route("/student-form-assignments", post(create_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments", get(get_assignments_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments", put(update_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments", delete(delete_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/review", put(review_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/assign", post(bulk_assign_forms_to_students).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/assign-to-school", post(assign_form_to_school_students).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(student_form_assignment_service)

        // Section 10 Portal APIs (JWT or API Key with parent isolation for JWT)
        .route("/parents/:parent_id/children", get(get_parent_children).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/parents/:parent_id/children/:child_id/profile", get(get_child_profile).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/parents/:parent_id/children/:child_id/forms", get(get_child_forms).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/classrooms/:id", get(get_classroom_details).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms/:id/forms", get(get_classroom_forms).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms/:id/forms", post(assign_classroom_form).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms/:id/forms/:form_id", delete(remove_classroom_form).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/parents/:parent_id", get(get_parent_profile).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/children/:child_id", get(get_child_demographics).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(portal_service)

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(cors);

    Ok(app)
}
