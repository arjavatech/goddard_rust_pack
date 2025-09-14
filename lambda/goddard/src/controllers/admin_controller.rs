use axum::{
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use crate::models::schema::{DashboardOverview, ApiResponse};
// use crate::db::admin;

pub async fn get_dashboard_overview(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
) -> Result<Json<ApiResponse<DashboardOverview>>, StatusCode> {
    // Create database connection
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost/goddard_db".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // TODO: Re-enable when database module is fixed
    Err(StatusCode::NOT_IMPLEMENTED)
}