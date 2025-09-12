use lambda_web::http::{Method, StatusCode, Uri};
use lambda_web::{Request, Body};
use serde_json::Value;
use std::collections::HashMap;

// Import the handlers we want to test
use hello_world_lambda::handlers::hello;

/// Helper function to create test requests
fn create_test_request(method: Method, uri: &str) -> Request {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::Empty)
        .unwrap()
}

/// Helper function to parse response body as JSON
async fn parse_response_json(response: lambda_web::Response<Body>) -> Value {
    let body_bytes = match response.into_body() {
        Body::Text(text) => text.into_bytes(),
        Body::Binary(bytes) => bytes,
        Body::Empty => vec![],
    };
    
    serde_json::from_slice(&body_bytes).expect("Failed to parse JSON response")
}

#[tokio::test]
async fn test_hello_world_endpoint() {
    // Test the root endpoint
    let request = create_test_request(Method::GET, "/");
    let response = hello::hello_world(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Check content type header
    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "application/json");
    
    // Parse and check JSON response
    let json_body = parse_response_json(response).await;
    
    assert_eq!(json_body["success"], true);
    assert_eq!(json_body["data"]["greeting"], "Hello, World!");
    assert!(json_body["timestamp"].is_string());
}

#[tokio::test]
async fn test_health_check_endpoint() {
    let request = create_test_request(Method::GET, "/health");
    let response = hello::health_check(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json_body = parse_response_json(response).await;
    
    assert_eq!(json_body["success"], true);
    assert_eq!(json_body["data"]["status"], "healthy");
    assert!(json_body["data"]["version"].is_string());
    assert!(json_body["data"]["uptime"].is_string());
}

#[tokio::test]
async fn test_hello_name_endpoint() {
    // Test with a specific name
    let request = create_test_request(Method::GET, "/hello/Rust");
    let response = hello::hello_name(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json_body = parse_response_json(response).await;
    
    assert_eq!(json_body["success"], true);
    assert_eq!(json_body["data"]["greeting"], "Hello, Rust!");
    assert_eq!(json_body["data"]["name"], "Rust");
}

#[tokio::test]
async fn test_hello_name_with_special_characters() {
    // Test with special characters in name
    let request = create_test_request(Method::GET, "/hello/Test-User_123");
    let response = hello::hello_name(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json_body = parse_response_json(response).await;
    
    assert_eq!(json_body["success"], true);
    assert_eq!(json_body["data"]["greeting"], "Hello, Test-User_123!");
    assert_eq!(json_body["data"]["name"], "Test-User_123");
}

#[tokio::test]
async fn test_cors_headers() {
    let request = create_test_request(Method::GET, "/");
    let response = hello::hello_world(request).await.unwrap();
    
    // Check CORS headers are present
    assert!(response.headers().contains_key("access-control-allow-origin"));
    assert!(response.headers().contains_key("access-control-allow-methods"));
    assert!(response.headers().contains_key("access-control-allow-headers"));
    
    // Check CORS header values
    let cors_origin = response.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(cors_origin, "*");
}

#[tokio::test]
async fn test_options_handler() {
    let request = create_test_request(Method::OPTIONS, "/");
    let response = hello::options_handler(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    
    // Check CORS headers for preflight
    assert!(response.headers().contains_key("access-control-allow-origin"));
    assert!(response.headers().contains_key("access-control-allow-methods"));
    assert!(response.headers().contains_key("access-control-max-age"));
}

#[tokio::test]
async fn test_response_structure() {
    let request = create_test_request(Method::GET, "/");
    let response = hello::hello_world(request).await.unwrap();
    
    let json_body = parse_response_json(response).await;
    
    // Verify the API response structure
    assert!(json_body.get("success").is_some());
    assert!(json_body.get("message").is_some());
    assert!(json_body.get("data").is_some());
    assert!(json_body.get("timestamp").is_some());
    
    // Verify data structure
    let data = &json_body["data"];
    assert!(data.get("greeting").is_some());
    assert!(data.get("name").is_some() || data["name"].is_null());
}

#[tokio::test]
async fn test_concurrent_requests() {
    // Test multiple concurrent requests to ensure thread safety
    let mut handles = vec![];
    
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let request = create_test_request(Method::GET, &format!("/hello/User{}", i));
            let response = hello::hello_name(request).await.unwrap();
            
            assert_eq!(response.status(), StatusCode::OK);
            
            let json_body = parse_response_json(response).await;
            assert_eq!(json_body["success"], true);
            assert_eq!(json_body["data"]["greeting"], format!("Hello, User{}!", i));
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Performance test - measure response time
#[tokio::test]
async fn test_response_performance() {
    let start = std::time::Instant::now();
    
    let request = create_test_request(Method::GET, "/");
    let response = hello::hello_world(request).await.unwrap();
    
    let duration = start.elapsed();
    
    assert_eq!(response.status(), StatusCode::OK);
    // Response should be fast (under 100ms for local test)
    assert!(duration.as_millis() < 100, "Response took too long: {:?}", duration);
}