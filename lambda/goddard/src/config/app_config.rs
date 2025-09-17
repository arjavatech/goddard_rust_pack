use serde::Deserialize;
use std::env;
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration: u64,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(AppConfig {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8000".to_string())
                    .parse()
                    .map_err(|_| AppError::Internal("Invalid port configuration".to_string()))?,
                cors_origins: env::var("CORS_ORIGINS")
                    .unwrap_or_else(|_| "*".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .map_err(|_| AppError::Internal("DATABASE_URL environment variable is required".to_string()))?,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
                connection_timeout: env::var("DB_CONNECTION_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
            auth: AuthConfig {
                jwt_secret: env::var("JWT_SECRET")
                    .map_err(|_| AppError::Internal("JWT_SECRET environment variable is required".to_string()))?,
                jwt_expiration: env::var("JWT_EXPIRATION")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .unwrap_or(86400),
                api_key: env::var("OWNER_API_KEY")
                    .map_err(|_| AppError::Internal("OWNER_API_KEY environment variable is required".to_string()))?,
            },
            logging: LoggingConfig {
                level: env::var("LOG_LEVEL")
                    .unwrap_or_else(|_| "info".to_string()),
                file_path: env::var("LOG_FILE").ok(),
            },
        })
    }
}