// Simple standalone test for tokio-postgres connection
use tokio_postgres::{NoTls, Error};
use deadpool_postgres::{Config, Runtime};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = "postgresql://postgres:Arjava%402024@db.fxsjcrwsnnowlovcnddz.supabase.co:5432/postgres";

    println!("Testing direct connection...");

    // Test 1: Direct connection
    let (client, connection) = tokio_postgres::connect(config, NoTls).await?;

    // The connection object needs to be spawned to be processed
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Test a simple query
    let row = client.query_one("SELECT 1 as test", &[]).await?;
    let value: i32 = row.get(0);
    println!("✓ Direct connection works! Query result: {}", value);

    println!("Testing connection pool...");

    // Test 2: Connection pool using deadpool-postgres
    let mut pool_config = Config::new();

    // Parse the URL for pool configuration
    if let Ok(parsed) = url::Url::parse(config) {
        if let Some(host) = parsed.host_str() {
            pool_config.host = Some(host.to_string());
        }
        pool_config.port = parsed.port();
        pool_config.user = Some(parsed.username().to_string());
        // URL decode the password to handle special characters
        pool_config.password = parsed.password().map(|p| {
            percent_encoding::percent_decode_str(p)
                .decode_utf8()
                .unwrap_or_default()
                .to_string()
        });
        pool_config.dbname = Some(parsed.path().trim_start_matches('/').to_string());
    }

    pool_config.pool = Some(deadpool_postgres::PoolConfig::new(16));

    let pool = pool_config.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create pool");

    // Test getting connection from pool
    let pool_client = pool.get().await
        .expect("Failed to get connection from pool");

    let row2 = pool_client.query_one("SELECT 2 as test", &[]).await?;
    let value2: i32 = row2.get(0);
    println!("✓ Connection pool works! Query result: {}", value2);

    println!("Testing JSON and UUID support...");

    // Test UUID and JSON types
    let test_uuid = uuid::Uuid::new_v4();
    let test_json = serde_json::json!({"test": "data", "number": 123});

    let row3 = pool_client.query_one(
        "SELECT $1::uuid as test_uuid, $2::jsonb as test_json",
        &[&test_uuid, &test_json]
    ).await?;

    let returned_uuid: uuid::Uuid = row3.get(0);
    let returned_json: serde_json::Value = row3.get(1);

    println!("✓ UUID and JSON support works!");
    println!("  UUID: {} -> {}", test_uuid, returned_uuid);
    println!("  JSON: {} -> {}", test_json, returned_json);

    println!("\n🎉 All tests passed! tokio-postgres with deadpool connection pooling is working correctly.");

    Ok(())
}