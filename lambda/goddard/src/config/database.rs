use sqlx::PgPool;
use std::env;
use crate::error::AppError;

pub struct DatabaseConfig {
    pub url: String,
}

impl DatabaseConfig {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn from_env() -> Self {
        let url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        Self::new(url)
    }

    pub async fn create_pool(&self) -> Result<PgPool, AppError> {
        PgPool::connect(&self.url)
            .await
            .map_err(|e| AppError::Database(format!("Failed to connect to database: {}", e)))
    }
}

// Global database instance - will be initialized at startup
use std::sync::OnceLock;

pub static DB_POOL: OnceLock<PgPool> = OnceLock::new();

pub async fn initialize_database() -> Result<(), AppError> {
    let config = DatabaseConfig::from_env();
    let pool = config.create_pool().await?;
    DB_POOL.set(pool)
        .map_err(|_| AppError::Internal("Failed to set database pool".to_string()))?;
    Ok(())
}

pub fn get_db_pool() -> &'static PgPool {
    DB_POOL.get().expect("Database pool not initialized")
}