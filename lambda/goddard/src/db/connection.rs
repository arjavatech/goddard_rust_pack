use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;
use std::env;
use std::sync::Arc;

pub async fn create_pool() -> Result<Arc<Pool>, tokio_postgres::Error> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let mut config = Config::new();

    // Parse the connection URL
    if let Ok(parsed) = url::Url::parse(&database_url) {
        if let Some(host) = parsed.host_str() {
            config.host = Some(host.to_string());
        }
        config.port = parsed.port();
        config.user = Some(parsed.username().to_string());
        config.password = parsed.password().map(|p| p.to_string());
        config.dbname = Some(parsed.path().trim_start_matches('/').to_string());
    } else {
        panic!("Invalid DATABASE_URL format");
    }

    // Connection pool settings
    config.pool = Some(deadpool_postgres::PoolConfig::new(5));

    let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create pool");

    Ok(Arc::new(pool))
}