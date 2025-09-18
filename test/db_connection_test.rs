// Simple test to verify tokio-postgres connection works
use std::env;
use tokio_postgres::{NoTls, Error};
use deadpool_postgres::{Config, Runtime};

#[tokio::test]
async fn test_database_connection() {
    // Set up environment variables for testing
    env::set_var("DATABASE_URL", "postgresql://postgres:Arjava%402024@db.fxsjcrwsnnowlovcnddz.supabase.co:5432/postgres");

    let database_url = env::var("DATABASE_URL").unwrap();

    // Test direct connection
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .expect("Failed to connect directly");

    // Spawn the connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Test a simple query
    let row = client.query_one("SELECT 1 as test", &[])
        .await
        .expect("Failed to execute query");

    let value: i32 = row.get(0);
    assert_eq!(value, 1);

    println!("✓ Direct connection test passed");
}

#[tokio::test]
async fn test_connection_pool() {
    // Set up environment variables for testing
    env::set_var("DATABASE_URL", "postgresql://postgres:Arjava%402024@db.fxsjcrwsnnowlovcnddz.supabase.co:5432/postgres");

    let database_url = env::var("DATABASE_URL").unwrap();

    // Parse URL and create pool config
    let mut config = Config::new();
    if let Ok(parsed) = url::Url::parse(&database_url) {
        if let Some(host) = parsed.host_str() {
            config.host = Some(host.to_string());
        }
        config.port = parsed.port();
        config.user = Some(parsed.username().to_string());
        config.password = parsed.password().map(|p| p.to_string());
        config.dbname = Some(parsed.path().trim_start_matches('/').to_string());
    }

    config.pool = Some(deadpool_postgres::PoolConfig::new(5));

    let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create pool");

    // Test getting connection from pool
    let client = pool.get().await.expect("Failed to get connection from pool");

    // Test a simple query
    let row = client.query_one("SELECT 2 as test", &[])
        .await
        .expect("Failed to execute query");

    let value: i32 = row.get(0);
    assert_eq!(value, 2);

    println!("✓ Connection pool test passed");
}