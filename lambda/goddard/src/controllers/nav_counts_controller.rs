use axum::{extract::State, response::IntoResponse, Extension};
use std::sync::Arc;

use crate::{
    dao::NavCountsDao,
    error::AppError,
    middleware::auth::AuthContext,
    models::schema::UserRole,
    utils::ResponseUtils,
};

/// GET /nav-counts — admin / superadmin only
pub async fn get_admin_nav_counts(
    Extension(auth): Extension<AuthContext>,
    State(dao): State<Arc<NavCountsDao>>,
) -> Result<impl IntoResponse, AppError> {
    let counts = dao.get_admin_counts(auth.school_id, auth.user_id).await?;
    Ok(ResponseUtils::success(counts))
}

/// GET /my-nav-counts — parent or employee (any authenticated user)
pub async fn get_my_nav_counts(
    Extension(auth): Extension<AuthContext>,
    State(dao): State<Arc<NavCountsDao>>,
) -> Result<impl IntoResponse, AppError> {
    let counts = match auth.role {
        UserRole::Teacher => dao.get_employee_counts(auth.user_id, auth.school_id).await?,
        _ => dao.get_parent_counts(auth.user_id, auth.school_id).await?,
    };
    Ok(ResponseUtils::success(counts))
}
