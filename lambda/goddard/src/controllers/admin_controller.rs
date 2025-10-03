use axum::{
    extract::{State, Extension, Query},
    response::IntoResponse,
    http::StatusCode,
};
use std::sync::Arc;

use crate::{
    middleware::auth::AuthContext,
    services::admin_service::AdminService,
    models::admin::GetAdminDashboardMetricsRequest,
    utils::ResponseUtils,
    error::AppError,
};

/// GET /admin/dashboard-metrics?school_id={uuid}
/// Get admin dashboard metrics (JWT protected - Admin/SuperAdmin)
pub async fn get_admin_dashboard_metrics(
    Extension(auth): Extension<AuthContext>,
    State(admin_service): State<Arc<AdminService>>,
    Query(query): Query<GetAdminDashboardMetricsRequest>,
) -> Result<impl IntoResponse, AppError> {
    println!("[DEBUG] Getting dashboard metrics - User: {}, Role: {:?}, School: {}",
        auth.email, auth.role, query.school_id);

    let response = admin_service.get_dashboard_metrics(query).await?;
    Ok(ResponseUtils::success(response))
}
