use deadpool_postgres::{Config, Pool, Runtime, PoolConfig};
use tokio_postgres::NoTls;
use std::env;
use std::time::Duration;
use percent_encoding;
use crate::error::AppError;

#[derive(Debug)]
pub struct DatabaseConfig {
    pub url: String,
}

impl DatabaseConfig {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn from_env() -> Result<Self, AppError> {
        let url = env::var("DATABASE_URL")
            .map_err(|_| AppError::Database(
                "DATABASE_URL environment variable must be set. \
                 For local: add to .env file. \
                 For Fly.io: run 'fly secrets set DATABASE_URL=<your-db-url>'. \
                 For AWS Lambda: set in environment variables.".to_string()
            ))?;
        Ok(Self::new(url))
    }

    pub fn create_pool(&self) -> Result<Pool, AppError> {
        let mut config = Config::new();

        // Parse the connection URL
        let url = &self.url;
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                config.host = Some(host.to_string());
            }
            config.port = parsed.port();
            config.user = Some(parsed.username().to_string());
            // URL decode the password to handle special characters like @ in passwords
            config.password = parsed.password().map(|p| {
                percent_encoding::percent_decode_str(p)
                    .decode_utf8()
                    .unwrap_or_default()
                    .to_string()
            });
            config.dbname = Some(parsed.path().trim_start_matches('/').to_string());

            // Extract connect_timeout from query parameters if present
            for (key, value) in parsed.query_pairs() {
                if key == "connect_timeout" {
                    if let Ok(timeout_secs) = value.parse::<u64>() {
                        config.connect_timeout = Some(Duration::from_secs(timeout_secs));
                    }
                }
            }
        } else {
            return Err(AppError::Database("Invalid DATABASE_URL format".to_string()));
        }

        // Fly.io optimized connection pool settings
        // Pool size of 6 per machine (2 machines = 12 total) for persistent server deployment
        // Allows for nested transactions without deadlocks while handling concurrent requests
        // (e.g., create_child opens transaction, create_enrollment needs separate connection)
        let mut pool_config = PoolConfig::new(6);

        // Timeouts optimized for Supabase Session Pooler on Fly.io
        // Session Pooler maintains persistent connections across requests
        pool_config.timeouts.wait = Some(Duration::from_secs(5)); // Max wait time for connection from pool
        pool_config.timeouts.create = Some(Duration::from_secs(10)); // Max time to create new connection
        pool_config.timeouts.recycle = Some(Duration::from_secs(5)); // Max time to recycle connection

        config.pool = Some(pool_config);

        // Fly.io Session Pooler Configuration
        // Uses Verified recycling method to enable prepared statements and connection validation
        config.manager = Some(deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Verified,
        });

        config.create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| AppError::Database(format!("Failed to create connection pool: {}", e)))
    }
}

// Global database instance - will be initialized at startup
use std::sync::OnceLock;

pub static DB_POOL: OnceLock<Pool> = OnceLock::new();

pub async fn initialize_database() -> Result<(), AppError> {
    let config = DatabaseConfig::from_env()?;
    let pool = config.create_pool()?;

    // Test the connection
    let _client = pool.get()
        .await
        .map_err(|e| AppError::Database(format!("Failed to get connection from pool: {}", e)))?;

    DB_POOL.set(pool)
        .map_err(|_| AppError::Internal("Failed to set database pool".to_string()))?;
    Ok(())
}

pub fn get_db_pool() -> &'static Pool {
    DB_POOL.get().expect("Database pool not initialized")
}