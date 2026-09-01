use axum::{
    middleware as axum_middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use lambda_http::run;

mod config;
mod controllers;
mod dao;
mod db;
mod error;
mod middleware;
mod models;
mod services;
mod utils;

use config::database::{get_db_pool, initialize_database};
use controllers::{
    admin_controller::get_admin_dashboard_metrics,
    auth_verification_controller::{
        clear_auth_table, create_invitation, create_invitation_enhanced, create_superadmin,
        debug_auth_users, delete_admin_user, forgot_password, get_admins_by_school,
        get_auth_verification_status, get_current_user_profile, get_invitation_summary,
        get_users_by_school_and_role, resend_admin_invite, update_admin_user,
    },
    class_form_override_controller::{create_class_form_override, delete_class_form_override},
    classroom_controller::{
        create_classroom, delete_classroom, get_classrooms_by_school, update_classroom,
    },
    device_token_controller::{
        device_token_status, register_device_token, unregister_device_token,
    },
    document_request_controller::{
        complete_document_upload, create_document_request, document_assignment_history,
        document_file_url, document_recipients, document_review_queue, document_upload_intent,
        list_document_assignments, list_document_requests, my_document_assignments,
        publish_document_request, review_document_assignment, send_document_reminders,
    },
    email_controller::send_bulk_form_reminders,
    employee_controller::{
        activate_employee, assign_employee_form, assign_employee_form_to_school,
        bulk_create_employees, complete_employee_template_pdf_upload,
        create_employee_form_template, deactivate_employee, delete_employee_form_assignment,
        delete_employee_form_template, employee_form_submission_webhook,
        employee_template_pdf_upload_intent, employee_template_pdf_url, get_current_employee,
        get_employee_by_id, get_employee_form_assignments, get_employee_form_review_queue,
        get_employee_form_templates, get_employee_forms, get_employees, invite_employee,
        remove_employee_template_pdf, resend_employee_invite, review_employee_form_assignment,
        send_bulk_employee_form_reminders, update_employee, update_employee_form_template,
    },
    enrollment_controller::{
        activate_invite, activate_parent, add_child, bulk_add_secondary_parents,
        bulk_import_families, bulk_promote_enrollments, create_parent_invite, deactivate_parent,
        edit_class_transition, get_class_based_enrollments, get_class_wise_count,
        get_enrollment_children_with_forms, get_parent_details_by_school, get_school_forms,
        promote_enrollment, resend_parent_confirmation, update_child_status,
    },
    expense_controller::{create_expense, list_expenses},
    form_submission_controller::{
        create_form_submission_webhook, get_form_resume_link, get_form_submission_by_id,
        get_form_submission_versions, get_latest_form_submission, update_form_submission_status,
    },
    form_template_controller::{
        complete_form_template_pdf_upload, create_form_template, delete_form_template,
        form_template_pdf_upload_intent, form_template_pdf_url, get_form_templates_by_school,
        remove_form_template_pdf, update_form_template,
    },
    hello_controller::{health_check, hello_name, hello_world},
    notification_controller::{list_notifications, mark_all_read, mark_read, unread_count},
    parent_details_controller::get_parent_details_by_id,
    portal_controller::{
        assign_classroom_form, get_child_demographics, get_child_forms, get_child_profile,
        get_classroom_details, get_classroom_forms, get_parent_children, get_parent_profile,
        remove_classroom_form,
    },
    request_controller::{
        create_request, delete_request, list_requests, pay_request,
        update_expected_completion_date, update_request, update_request_status,
    },
    school_controller::{
        create_school, create_school_with_owner, delete_school, get_all_schools,
        get_all_schools_with_owners, get_request_settings, get_school_with_owner,
        update_request_settings, update_school,
    },
    student_form_assignment_controller::{
        assign_form_to_class_students, assign_form_to_school_students,
        bulk_assign_forms_to_students, create_student_form_assignment,
        delete_student_form_assignment, download_enrollment_forms_zip, get_assignments_by_school,
        get_student_form_review_queue, update_student_form_assignment,
    },
    student_form_assignment_review_controller::review_student_form_assignment,
        taptime_mapping_controller::{
            attendance_users, available_taptime_users, create_mapping, database_diagnostics, integration_status, mapping_users, reconcile_users,
        redeem_pairing_code, setup_status, sync_access, taptime_settings, update_taptime_settings,
    },
};
use dao::{
    AdminDao, AuthDao, ClassFormOverrideDao, ClassroomDao, DeviceTokenDao, DocumentRequestDao,
    EmployeeDao, EmployeeFormAssignmentDao, EmployeeFormSubmissionDao, EmployeeFormTemplateDao,
    EnrollmentDao, FormSubmissionDao, FormTemplateDao, NotificationDao, PortalDao, RequestDao,
    SchoolDao, StudentFormAssignmentDao, TapTimeMappingDao,
};
use middleware::auth::{
    api_key_middleware, jwt_or_api_key_admin_only, jwt_or_api_key_middleware,
    jwt_or_api_key_superadmin_only,
};
use middleware::{cors::add_cors_headers, request_id::request_id_middleware};
use services::{
    AdminService, AuthService, ClassFormOverrideService, ClassroomService, DocumentRequestService,
    EmailService, EmployeeService, EnrollmentService, FilloutService, FormSubmissionService,
    FormTemplateService, NotificationPushTrigger, NotificationService, PortalService,
    RequestService, SchoolService, StudentFormAssignmentService, SupabaseClient,
    TapTimeMappingService, TapTimeService, UploadService,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Log the effective, non-secret email routing configuration at boot. This is
    // intentionally read after dotenv so it reveals values inherited from the
    // process environment as well as those loaded from the env file.
    let email_provider = std::env::var("EMAIL_PROVIDER").unwrap_or_else(|_| "smtp".to_string());
    let email_from = std::env::var("EMAIL_FROM")
        .unwrap_or_else(|_| "Goddard Schools <no-reply@arjavatech.com>".to_string());
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.zoho.com".to_string());
    let smtp_port = std::env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string());
    println!(
        "[EmailConfig] provider={} from=\"{}\" smtp_host={} smtp_port={}",
        email_provider, email_from, smtp_host, smtp_port
    );

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
    let notification_dao = NotificationDao::new(pool.clone());
    let device_token_dao = Arc::new(DeviceTokenDao::new(pool.clone()));
    let employee_dao = EmployeeDao::new(pool.clone());
    let employee_form_template_dao = EmployeeFormTemplateDao::new(pool.clone());
    let employee_form_assignment_dao = EmployeeFormAssignmentDao::new(pool.clone());
    let employee_form_submission_dao = EmployeeFormSubmissionDao::new(pool.clone());
    let request_dao = RequestDao::new(pool.clone());
    let document_request_dao = DocumentRequestDao::new(pool.clone());
    let taptime_mapping_dao = TapTimeMappingDao::new(pool.clone());
    let taptime_service = TapTimeService::from_env()?;

    // Initialize email service first (SupabaseClient depends on it)
    let email_service = Arc::new(EmailService::new());

    // Initialize Supabase client
    let supabase_client = SupabaseClient::new(email_service.clone())?;

    let taptime_mapping_service = Arc::new(TapTimeMappingService::new(
        employee_dao.clone(),
        auth_dao.clone(),
        taptime_mapping_dao,
        taptime_service.clone(),
        supabase_client.clone(),
        school_dao.clone(),
    ));

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

    let notification_push_trigger = NotificationPushTrigger::from_environment().await;
    let notification_service = Arc::new(NotificationService::new(
        notification_dao,
        school_dao.clone(),
        notification_push_trigger,
    ));
    let auth_service = Arc::new(AuthService::new(
        auth_dao.clone(),
        school_dao.clone(),
        supabase_client.clone(),
        notification_service.clone(),
        taptime_mapping_service.clone(),
    ));
    let school_service = Arc::new(SchoolService::new(
        school_dao.clone(),
        supabase_client.clone(),
        auth_dao.clone(),
    ));
    let classroom_service = Arc::new(ClassroomService::new(
        classroom_dao,
        school_dao.clone(),
        notification_service.clone(),
    ));
    let upload_service = Arc::new(UploadService::new().await);
    let form_template_service = Arc::new(FormTemplateService::new(
        form_template_dao,
        school_dao.clone(),
        notification_service.clone(),
        upload_service.clone(),
    ));
    let class_form_override_service =
        Arc::new(ClassFormOverrideService::new(class_form_override_dao));
    let enrollment_service = Arc::new(EnrollmentService::new(
        enrollment_dao,
        school_dao.clone(),
        supabase_client.clone(),
        email_service.clone(),
        notification_service.clone(),
    ));
    let form_submission_service = Arc::new(if let Some(fillout) = fillout_service {
        FormSubmissionService::new_with_fillout(
            form_submission_dao,
            fillout,
            notification_service.clone(),
            StudentFormAssignmentDao::new(pool.clone()),
        )
    } else {
        FormSubmissionService::new(
            form_submission_dao,
            notification_service.clone(),
            StudentFormAssignmentDao::new(pool.clone()),
        )
    });
    let student_form_assignment_service = Arc::new(StudentFormAssignmentService::new(
        student_form_assignment_dao,
        email_service.clone(),
        notification_service.clone(),
    ));
    let portal_service = Arc::new(PortalService::new(Arc::new(portal_dao)));
    let admin_service = Arc::new(AdminService::new(admin_dao));
    let request_service = Arc::new(RequestService::new(request_dao, upload_service.clone()));
    let document_request_service = Arc::new(DocumentRequestService::new(
        document_request_dao,
        upload_service.clone(),
        notification_service.clone(),
        email_service.clone(),
    ));

    let employee_service = Arc::new(EmployeeService::new(
        employee_dao,
        employee_form_template_dao,
        employee_form_assignment_dao,
        employee_form_submission_dao,
        auth_dao.clone(),
        school_dao.clone(),
        supabase_client.clone(),
        email_service.clone(),
        upload_service.clone(),
        taptime_mapping_service.clone(),
    ));

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .without_time()
        .init();

    let taptime_mapping_router = Router::new()
        .route(
            "/taptime/settings",
            get(taptime_settings).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only))
                .patch(update_taptime_settings).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/setup-status",
            get(setup_status).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/integration-status",
            get(integration_status).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/setup/redeem-linking-code",
            post(redeem_pairing_code).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/reconcile",
            post(reconcile_users).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/sync-access",
            post(sync_access).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/mapping-users",
            get(mapping_users).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/taptime/users",
            get(mapping_users).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/taptime/attendance-users",
            get(attendance_users).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/taptime/available-users",
            get(available_taptime_users)
                .layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        // TEMPORARY development diagnostic; protected exactly like mapping operations.
        .route(
            "/taptime/diagnostics/database",
            get(database_diagnostics)
                .layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/taptime/user-mappings",
            post(create_mapping).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .with_state(taptime_mapping_service);

    // Build the application router
    let app = Router::new()
        .merge(taptime_mapping_router)
        // Health and Info Routes
        .route("/", get(hello_world))
        .route("/health", get(health_check))
        .route("/hello/:name", get(hello_name))
        // Authorization Verification Routes (Legacy)
        .route(
            "/auth/verification-status",
            get(get_auth_verification_status),
        )
        .route("/auth/invitation-summary", get(get_invitation_summary))
        .route(
            "/auth/invite-create",
            post(create_invitation).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/auth/invite-create-enhanced",
            post(create_invitation_enhanced)
                .layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/auth/create-superadmin",
            post(create_superadmin).layer(axum_middleware::from_fn(api_key_middleware)),
        )
        .route("/auth/clear-table", delete(clear_auth_table))
        .route("/auth/debug-users", get(debug_auth_users))
        .route("/auth/users/filter", get(get_users_by_school_and_role))
        .route("/auth/forgot-password", post(forgot_password))
        .route(
            "/auth/admin-resend-invite",
            post(resend_admin_invite)
                .layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route("/users/me", get(get_current_user_profile))
        .route(
            "/users/admin",
            get(get_admins_by_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only))
                .merge(
                    put(update_admin_user)
                        .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
                )
                .merge(
                    delete(delete_admin_user)
                        .layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
                ),
        )
        .with_state(auth_service)
        // School Management APIs (Admin JWT or API Key)
        .route(
            "/schools/with-owner",
            post(create_school_with_owner).layer(axum_middleware::from_fn(api_key_middleware)),
        )
        .route(
            "/schools/with-owners",
            get(get_all_schools_with_owners).layer(axum_middleware::from_fn(api_key_middleware)),
        )
        .route(
            "/schools/:id/owner",
            get(get_school_with_owner).layer(axum_middleware::from_fn(api_key_middleware)),
        )
        .route(
            "/schools",
            post(create_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route("/schools", get(get_all_schools)) // Public
        .route(
            "/schools",
            put(update_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/schools/:id",
            delete(delete_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/schools/:id/request-settings",
            get(get_request_settings).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/schools/:id/request-settings",
            patch(update_request_settings)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(school_service)
        // Classroom Management APIs (Admin JWT or API Key)
        .route(
            "/classrooms",
            post(create_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/classrooms",
            get(get_classrooms_by_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/classrooms",
            put(update_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/classrooms",
            delete(delete_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(classroom_service)
        // Form Templates Management APIs (Admin JWT or API Key)
        .route(
            "/form-templates",
            post(create_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates",
            get(get_form_templates_by_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates",
            put(update_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates",
            delete(delete_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates/:id/pdf/upload-intent",
            post(form_template_pdf_upload_intent)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates/:id/pdf/complete-upload",
            post(complete_form_template_pdf_upload)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates/:id/pdf",
            get(form_template_pdf_url).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-templates/:id/pdf",
            delete(remove_form_template_pdf)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(form_template_service)
        // Class Form Overrides Management APIs (Admin JWT or API Key)
        .route(
            "/class-form-overrides",
            post(create_class_form_override)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/class-form-overrides",
            delete(delete_class_form_override)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(class_form_override_service)
        // Admin Dashboard APIs (JWT protected - Admin/SuperAdmin)
        .route(
            "/admin/dashboard-metrics",
            get(get_admin_dashboard_metrics)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(admin_service)
        // Email APIs (JWT or API Key protected - Admin/SuperAdmin only)
        .route(
            "/emails/bulk-form-reminders",
            post(send_bulk_form_reminders)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(email_service)
        // In-app Notifications (JWT protected - current user's own notifications)
        .route(
            "/notifications",
            get(list_notifications).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/notifications/unread-count",
            get(unread_count).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/notifications/mark-all-read",
            patch(mark_all_read).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/notifications/:id/read",
            patch(mark_read).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .with_state(notification_service)
        // Enrollment Management APIs (Admin JWT or API Key)
        .route(
            "/enrollments/parent-invite",
            post(create_parent_invite).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/resend-confirmation",
            post(resend_parent_confirmation)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/add-child",
            post(add_child).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/parent-details-by-school",
            get(get_parent_details_by_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/parent/details",
            get(get_parent_details_by_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/children-forms",
            get(get_enrollment_children_with_forms)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments",
            get(get_school_forms).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/class-wise-count",
            get(get_class_wise_count).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/class-based-enrollments",
            get(get_class_based_enrollments)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/parent/:parent_id",
            get(get_parent_details_by_id)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/parent/:parent_id",
            delete(deactivate_parent).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/parent/:parent_id/activate",
            patch(activate_parent).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/children/:child_id/status",
            patch(update_child_status).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        // Class Transitions APIs (Admin JWT or API Key)
        .route(
            "/class-promotions/:enrollment_id",
            post(promote_enrollment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/class-promotions/bulk",
            post(bulk_promote_enrollments)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/class-transitions/:enrollment_id",
            patch(edit_class_transition).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route("/enrollments/activate/:token", get(activate_invite))
        .route(
            "/enrollments/bulk-import",
            post(bulk_import_families).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/bulk-secondary-parents",
            post(bulk_add_secondary_parents)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(enrollment_service)
        // Form Submissions Management APIs (Admin JWT or API Key)
        .route(
            "/form-submissions/webhook",
            post(create_form_submission_webhook),
        )
        .route(
            "/form-submissions/latest",
            get(get_latest_form_submission)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-submissions/versions",
            get(get_form_submission_versions)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-submissions/:submission_id",
            get(get_form_submission_by_id)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/form-submissions/:submission_id/status",
            put(update_form_submission_status)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments/:assignment_id/resume-link",
            get(get_form_resume_link).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .with_state(form_submission_service)
        // Student Form Assignments Management APIs (Admin JWT or API Key)
        .route(
            "/student-form-assignments",
            post(create_student_form_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments/review-queue",
            get(get_student_form_review_queue)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments",
            get(get_assignments_by_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments",
            put(update_student_form_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments",
            delete(delete_student_form_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments/review",
            put(review_student_form_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments/assign",
            post(bulk_assign_forms_to_students)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments/assign-to-school",
            post(assign_form_to_school_students)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/student-form-assignments/assign-to-class",
            post(assign_form_to_class_students)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/enrollments/:enrollment_id/forms/download-zip",
            get(download_enrollment_forms_zip)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .with_state(student_form_assignment_service)
        // Document Requests: secure parent/employee upload and admin review workflow.
        .route(
            "/document-requests",
            post(create_document_request)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-requests",
            get(list_document_requests).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-request-recipients",
            get(document_recipients).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-requests/:id/publish",
            post(publish_document_request)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-assignments",
            get(list_document_assignments)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-assignments/reminders",
            post(send_document_reminders)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-assignments/review-queue",
            get(document_review_queue).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/my-document-assignments",
            get(my_document_assignments).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/document-assignments/:id/upload-intent",
            post(document_upload_intent).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/document-assignments/:id/complete-upload",
            post(complete_document_upload)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/document-assignments/:id/review",
            post(review_document_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/document-assignments/:id/history",
            get(document_assignment_history)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/document-submissions/:id/file",
            get(document_file_url).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .with_state(document_request_service)
        // Section 10 Portal APIs (JWT or API Key with parent isolation for JWT)
        .route(
            "/parents/:parent_id/children",
            get(get_parent_children).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/parents/:parent_id/children/:child_id/profile",
            get(get_child_profile).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/parents/:parent_id/children/:child_id/forms",
            get(get_child_forms).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/classrooms/:id",
            get(get_classroom_details).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/classrooms/:id/forms",
            get(get_classroom_forms).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/classrooms/:id/forms",
            post(assign_classroom_form).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/classrooms/:id/forms/:form_id",
            delete(remove_classroom_form)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/parents/:parent_id",
            get(get_parent_profile).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/children/:child_id",
            get(get_child_demographics).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(portal_service)
        // Push notification device token registration (JWT only - per-user)
        .route(
            "/device-tokens",
            post(register_device_token).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/device-tokens",
            delete(unregister_device_token)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/device-tokens/status",
            get(device_token_status).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .with_state(device_token_dao)
        // Employee Management APIs
        .route(
            "/employees/invite",
            post(invite_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/:employee_id/resend-invite",
            post(resend_employee_invite).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/bulk",
            post(bulk_create_employees).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/me",
            get(get_current_employee).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/employees",
            get(get_employees).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/:employee_id",
            get(get_employee_by_id).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/employees/:employee_id",
            patch(update_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/:employee_id",
            delete(deactivate_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/:employee_id/activate",
            patch(activate_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employees/:employee_id/forms",
            get(get_employee_forms).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        // Employee Form Templates (Admin only)
        .route(
            "/employee-form-templates",
            post(create_employee_form_template)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates",
            get(get_employee_form_templates)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates",
            put(update_employee_form_template)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates",
            delete(delete_employee_form_template)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates/:id/pdf/upload-intent",
            post(employee_template_pdf_upload_intent)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates/:id/pdf/complete-upload",
            post(complete_employee_template_pdf_upload)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates/:id/pdf",
            get(employee_template_pdf_url)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-templates/:id/pdf",
            delete(remove_employee_template_pdf)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        // Employee Form Assignments (Admin manages; Employee reads)
        .route(
            "/employee-form-assignments",
            post(assign_employee_form).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-assignments/review-queue",
            get(get_employee_form_review_queue)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-assignments/assign-to-school",
            post(assign_employee_form_to_school)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-assignments",
            get(get_employee_form_assignments)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/employee-form-assignments",
            delete(delete_employee_form_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/employee-form-assignments/review",
            put(review_employee_form_assignment)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        // Fillout webhook for employee form submissions (public)
        .route(
            "/employee-form-submissions/webhook",
            post(employee_form_submission_webhook),
        )
        // Bulk employee form reminders
        .route(
            "/emails/bulk-employee-form-reminders",
            post(send_bulk_employee_form_reminders)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .with_state(employee_service)
        // Procurement — Requests & Expenses (single table, single service)
        .route(
            "/requests",
            get(list_requests).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/requests",
            post(create_request).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/requests/:id/status",
            patch(update_request_status).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/requests/:id/expected-completion-date",
            patch(update_expected_completion_date)
                .layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)),
        )
        .route(
            "/requests/:id",
            patch(update_request).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/requests/:id/pay",
            post(pay_request).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/requests/:id",
            delete(delete_request).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)),
        )
        .route(
            "/expenses",
            get(list_expenses).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .route(
            "/expenses",
            post(create_expense).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)),
        )
        .with_state(request_service)
        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)); // 10 MB

    Ok(app)
}
