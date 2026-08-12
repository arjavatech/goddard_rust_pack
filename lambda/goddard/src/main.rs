use axum::{
    middleware as axum_middleware,
    routing::{get, post, put, delete, patch},
    Router,
};
use lambda_http::run;

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
        delete_admin_user,
        forgot_password,
        resend_admin_invite
    },
    school_controller::{
        create_school, get_all_schools, update_school, delete_school, create_school_with_owner, get_school_with_owner, get_all_schools_with_owners
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
        create_parent_invite, resend_parent_confirmation, add_child, get_parent_details_by_school, get_enrollment_children_with_forms, get_school_forms, get_class_wise_count, get_class_based_enrollments, deactivate_parent, activate_parent, update_child_status, promote_enrollment, bulk_promote_enrollments, edit_class_transition, activate_invite, bulk_import_families, bulk_add_secondary_parents
    },
    parent_details_controller::{
        get_parent_details_by_id
    },
    form_submission_controller::{
        create_form_submission_webhook, get_latest_form_submission, get_form_submission_versions,
        get_form_submission_by_id, update_form_submission_status, get_form_resume_link
    },
    student_form_assignment_controller::{
        create_student_form_assignment, get_assignments_by_school, update_student_form_assignment, delete_student_form_assignment, bulk_assign_forms_to_students, assign_form_to_school_students, assign_form_to_class_students, download_enrollment_forms_zip
    },
    student_form_assignment_review_controller::{
        review_student_form_assignment
    },
    portal_controller::{
        get_parent_children, get_child_profile, get_child_forms,
        get_classroom_details, get_classroom_forms, assign_classroom_form, remove_classroom_form,
        get_parent_profile, get_child_demographics
    },
    admin_controller::{
        get_admin_dashboard_metrics
    },
    email_controller::{
        send_bulk_form_reminders
    },
    notification_controller::{
        list_notifications, unread_count, mark_read, mark_all_read,
    },
    device_token_controller::{
        register_device_token, unregister_device_token,
    },
    websocket_controller::{
        websocket_handler,
    },
    employee_controller::{
        invite_employee, get_employees, get_employee_by_id, update_employee,
        deactivate_employee, activate_employee, get_employee_forms, get_current_employee,
        create_employee_form_template, get_employee_form_templates,
        update_employee_form_template, delete_employee_form_template,
        assign_employee_form, get_employee_form_assignments,
        review_employee_form_assignment, delete_employee_form_assignment,
        employee_form_submission_webhook, send_bulk_employee_form_reminders,
    },
    request_controller::{
        list_requests, create_request, update_request_status, pay_request, delete_request,
    },
    expense_controller::{
        list_expenses, create_expense,
    },
};
use middleware::{request_id::request_id_middleware, cors::add_cors_headers};
use config::database::{initialize_database, get_db_pool};
use dao::{
    AuthDao, SchoolDao, ClassroomDao, FormTemplateDao, ClassFormOverrideDao, EnrollmentDao, FormSubmissionDao, StudentFormAssignmentDao, PortalDao, AdminDao, NotificationDao, DeviceTokenDao,
    EmployeeDao, EmployeeFormTemplateDao, EmployeeFormAssignmentDao, EmployeeFormSubmissionDao,
    RequestDao,
};
use services::{
    AuthService, SupabaseClient, SchoolService, ClassroomService, FormTemplateService, ClassFormOverrideService, EnrollmentService, FormSubmissionService, StudentFormAssignmentService, PortalService, FilloutService, AdminService, EmailService, NotificationService, FcmService, ConnectionRegistry,
    EmployeeService, RequestService, UploadService,
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
    let notification_dao = NotificationDao::new(pool.clone());
    let device_token_dao = Arc::new(DeviceTokenDao::new(pool.clone()));
    let employee_dao = EmployeeDao::new(pool.clone());
    let employee_form_template_dao = EmployeeFormTemplateDao::new(pool.clone());
    let employee_form_assignment_dao = EmployeeFormAssignmentDao::new(pool.clone());
    let employee_form_submission_dao = EmployeeFormSubmissionDao::new(pool.clone());
    let request_dao = RequestDao::new(pool.clone());

    // Initialize FCM service. Live when all three env vars are present; otherwise a
    // no-op stub that lets local dev / staging boot without Firebase configured.
    let fcm_service = match (
        std::env::var("FCM_PROJECT_ID").ok(),
        std::env::var("FCM_CLIENT_EMAIL").ok(),
        std::env::var("FCM_PRIVATE_KEY").ok(),
    ) {
        (Some(pid), Some(email), Some(key)) if !pid.is_empty() && !email.is_empty() && !key.is_empty() => {
            println!("[DEBUG] FCM service initialized (project={})", pid);
            Arc::new(FcmService::live(pid, email, key, device_token_dao.clone()))
        }
        _ => {
            println!("[WARN] FCM service disabled - missing FCM_PROJECT_ID / FCM_CLIENT_EMAIL / FCM_PRIVATE_KEY");
            Arc::new(FcmService::disabled())
        }
    };

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
    let email_service = Arc::new(EmailService::new());
    
    // ✅ NEW: Initialize connection registry for WebSocket
    let connection_registry = Arc::new(ConnectionRegistry::new());
    
    let notification_service = Arc::new(NotificationService::new(notification_dao, fcm_service.clone(), connection_registry.clone()));
    let auth_service = Arc::new(AuthService::new(auth_dao.clone(), school_dao.clone(), supabase_client.clone(), notification_service.clone()));
    let school_service = Arc::new(SchoolService::new(school_dao.clone(), supabase_client.clone(), auth_dao.clone()));
    let classroom_service = Arc::new(ClassroomService::new(classroom_dao, school_dao.clone(), notification_service.clone()));
    let form_template_service = Arc::new(FormTemplateService::new(form_template_dao, school_dao.clone(), notification_service.clone()));
    let class_form_override_service = Arc::new(ClassFormOverrideService::new(class_form_override_dao));
    let enrollment_service = Arc::new(EnrollmentService::new(enrollment_dao, school_dao.clone(), supabase_client.clone(), email_service.clone(), notification_service.clone()));
    let form_submission_service = Arc::new(
        if let Some(fillout) = fillout_service {
            FormSubmissionService::new_with_fillout(form_submission_dao, fillout, notification_service.clone(), StudentFormAssignmentDao::new(pool.clone()))
        } else {
            FormSubmissionService::new(form_submission_dao, notification_service.clone(), StudentFormAssignmentDao::new(pool.clone()))
        }
    );
    let student_form_assignment_service = Arc::new(StudentFormAssignmentService::new(student_form_assignment_dao, email_service.clone(), notification_service.clone()));
    let portal_service = Arc::new(PortalService::new(Arc::new(portal_dao)));
    let admin_service = Arc::new(AdminService::new(admin_dao));
    let upload_service = Arc::new(UploadService::new().await);
    let request_service = Arc::new(RequestService::new(request_dao, upload_service.clone()));

    let employee_service = Arc::new(EmployeeService::new(
        employee_dao,
        employee_form_template_dao,
        employee_form_assignment_dao,
        employee_form_submission_dao,
        auth_dao.clone(),
        school_dao.clone(),
        supabase_client.clone(),
        email_service.clone(),
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
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/admin-resend-invite", post(resend_admin_invite).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        .route("/users/me", get(get_current_user_profile))
        .route("/users/admin",
            get(get_admins_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only))
            .merge(put(update_admin_user).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
            .merge(delete(delete_admin_user).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        )
        .with_state(auth_service)

        // School Management APIs (Admin JWT or API Key)
        .route("/schools/with-owner", post(create_school_with_owner).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/schools/with-owners", get(get_all_schools_with_owners).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/schools/:id/owner", get(get_school_with_owner).layer(axum_middleware::from_fn(api_key_middleware)))
        .route("/schools", post(create_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/schools", get(get_all_schools)) // Public
        .route("/schools", put(update_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/schools/:id", delete(delete_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(school_service)

        // Classroom Management APIs (Admin JWT or API Key)
        .route("/classrooms", post(create_classroom).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/classrooms", get(get_classrooms_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
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

        // Email APIs (JWT or API Key protected - Admin/SuperAdmin only)
        .route("/emails/bulk-form-reminders", post(send_bulk_form_reminders).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(email_service)

        // WebSocket endpoint for real-time notifications
        .route(
            "/notifications/ws",
            get(websocket_handler)
                .layer(axum_middleware::from_fn(jwt_or_api_key_middleware))
        )
        .with_state((connection_registry.clone(), notification_service.clone()))

        // In-app Notifications (JWT protected - current user's own notifications)
        .route("/notifications", get(list_notifications).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/notifications/unread-count", get(unread_count).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/notifications/mark-all-read", patch(mark_all_read).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/notifications/:id/read", patch(mark_read).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .with_state(notification_service)

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
        .route("/class-promotions/bulk", post(bulk_promote_enrollments).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/class-transitions/:enrollment_id", patch(edit_class_transition).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/activate/:token", get(activate_invite))
        .route("/enrollments/bulk-import", post(bulk_import_families).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/bulk-secondary-parents", post(bulk_add_secondary_parents).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .with_state(enrollment_service)

        // Form Submissions Management APIs (Admin JWT or API Key)
        .route("/form-submissions/webhook", post(create_form_submission_webhook))
        .route("/form-submissions/latest", get(get_latest_form_submission).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-submissions/versions", get(get_form_submission_versions).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-submissions/:submission_id", get(get_form_submission_by_id).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/form-submissions/:submission_id/status", put(update_form_submission_status).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/:assignment_id/resume-link", get(get_form_resume_link).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .with_state(form_submission_service)

        // Student Form Assignments Management APIs (Admin JWT or API Key)
        .route("/student-form-assignments", post(create_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments", get(get_assignments_by_school).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments", put(update_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments", delete(delete_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/review", put(review_student_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/assign", post(bulk_assign_forms_to_students).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/assign-to-school", post(assign_form_to_school_students).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/student-form-assignments/assign-to-class", post(assign_form_to_class_students).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/enrollments/:enrollment_id/forms/download-zip", get(download_enrollment_forms_zip).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
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

        // Push notification device token registration (JWT only - per-user)
        .route("/device-tokens", post(register_device_token).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/device-tokens/:token", delete(unregister_device_token).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .with_state(device_token_dao)

        // Employee Management APIs
        .route("/employees/invite", post(invite_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employees/me", get(get_current_employee).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/employees", get(get_employees).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employees/:employee_id", get(get_employee_by_id).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/employees/:employee_id", patch(update_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employees/:employee_id", delete(deactivate_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employees/:employee_id/activate", patch(activate_employee).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employees/:employee_id/forms", get(get_employee_forms).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))

        // Employee Form Templates (Admin only)
        .route("/employee-form-templates", post(create_employee_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employee-form-templates", get(get_employee_form_templates).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employee-form-templates", put(update_employee_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employee-form-templates", delete(delete_employee_form_template).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))

        // Employee Form Assignments (Admin manages; Employee reads)
        .route("/employee-form-assignments", post(assign_employee_form).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employee-form-assignments", get(get_employee_form_assignments).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/employee-form-assignments", delete(delete_employee_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/employee-form-assignments/review", put(review_employee_form_assignment).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))

        // Fillout webhook for employee form submissions (public)
        .route("/employee-form-submissions/webhook", post(employee_form_submission_webhook))

        // Bulk employee form reminders
        .route("/emails/bulk-employee-form-reminders", post(send_bulk_employee_form_reminders).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))

        .with_state(employee_service)

        // Procurement — Requests & Expenses (single table, single service)
        .route("/requests", get(list_requests).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/requests", post(create_request).layer(axum_middleware::from_fn(jwt_or_api_key_middleware)))
        .route("/requests/:id/status", patch(update_request_status).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/requests/:id/pay", post(pay_request).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        .route("/requests/:id", delete(delete_request).layer(axum_middleware::from_fn(jwt_or_api_key_admin_only)))
        .route("/expenses", get(list_expenses).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        .route("/expenses", post(create_expense).layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only)))
        .with_state(request_service)

        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(axum_middleware::from_fn(add_cors_headers))
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)); // 10 MB

    Ok(app)
}
