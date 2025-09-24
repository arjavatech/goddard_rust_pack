use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;
use std::env;
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

    pub fn from_env() -> Self {
        let url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        Self::new(url)
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

            // Add connection timeout and keepalive settings
            config.manager = Some(deadpool_postgres::ManagerConfig {
                recycling_method: deadpool_postgres::RecyclingMethod::Fast,
            });
            config.connect_timeout = Some(std::time::Duration::from_secs(5));
        } else {
            return Err(AppError::Database("Invalid DATABASE_URL format".to_string()));
        }

        // Connection pool settings with timeouts
        config.pool = Some(deadpool_postgres::PoolConfig::new(16));

        config.create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| AppError::Database(format!("Failed to create connection pool: {}", e)))
    }
}

// Global database instance - will be initialized at startup
use std::sync::OnceLock;

pub static DB_POOL: OnceLock<Pool> = OnceLock::new();

pub async fn initialize_database() -> Result<(), AppError> {
    let config = DatabaseConfig::from_env();
    let pool = config.create_pool()?;

    // Test the connection with timeout
    let timeout_duration = std::time::Duration::from_secs(5);
    let get_connection = pool.get();

    match tokio::time::timeout(timeout_duration, get_connection).await {
        Ok(Ok(_client)) => {
            // Connection successful
        },
        Ok(Err(e)) => {
            return Err(AppError::Database(format!("Failed to get connection from pool: {}", e)));
        },
        Err(_) => {
            return Err(AppError::Database("Database connection timeout - please check DATABASE_URL and ensure database is accessible".to_string()));
        }
    }

    DB_POOL.set(pool)
        .map_err(|_| AppError::Internal("Failed to set database pool".to_string()))?;
    Ok(())
}

pub fn get_db_pool() -> &'static Pool {
    DB_POOL.get().expect("Database pool not initialized")
}