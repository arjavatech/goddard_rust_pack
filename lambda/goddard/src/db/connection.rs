use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::sync::Arc;

pub async fn create_pool() -> Result<Arc<PgPool>, sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    Ok(Arc::new(pool))
}