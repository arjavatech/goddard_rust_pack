use hello_world_lambda::{ApiResponse, HealthData, HelloData, extract_path_params};
use std::collections::HashMap;

#[test]
fn test_api_response_success_creation() {
    let data = HelloData {
        greeting: "Hello, Test!".to_string(),
        name: Some("Test".to_string()),
    };
    
    let response = ApiResponse::success(data, "Test success");
    
    assert!(response.success);
    assert_eq!(response.message, "Test success");
    assert!(response.data.is_some());
    assert!(response.timestamp.len() > 0);
    
    let response_data = response.data.unwrap();
    assert_eq!(response_data.greeting, "Hello, Test!");
    assert_eq!(response_data.name, Some("Test".to_string()));
}

#[test]
fn test_api_response_error_creation() {
    let response = ApiResponse::<()>::error("Test error message");
    
    assert!(!response.success);
    assert_eq!(response.message, "Test error message");
    assert!(response.data.is_none());
    assert!(response.timestamp.len() > 0);
}

#[test]
fn test_health_data_creation() {
    let health_data = HealthData {
        status: "healthy".to_string(),
        version: "1.0.0".to_string(),
        uptime: "3600".to_string(),
    };
    
    assert_eq!(health_data.status, "healthy");
    assert_eq!(health_data.version, "1.0.0");
    assert_eq!(health_data.uptime, "3600");
}

#[test]
fn test_hello_data_creation() {
    let hello_data = HelloData {
        greeting: "Hello, World!".to_string(),
        name: None,
    };
    
    assert_eq!(hello_data.greeting, "Hello, World!");
    assert_eq!(hello_data.name, None);
    
    let hello_data_with_name = HelloData {
        greeting: "Hello, Rust!".to_string(),
        name: Some("Rust".to_string()),
    };
    
    assert_eq!(hello_data_with_name.greeting, "Hello, Rust!");
    assert_eq!(hello_data_with_name.name, Some("Rust".to_string()));
}

#[test]
fn test_extract_path_params_simple() {
    let params = extract_path_params("/hello/World", "/hello/{name}");
    
    assert_eq!(params.len(), 1);
    assert_eq!(params.get("name"), Some(&"World".to_string()));
}

#[test]
fn test_extract_path_params_multiple() {
    let params = extract_path_params("/users/123/posts/456", "/users/{user_id}/posts/{post_id}");
    
    assert_eq!(params.len(), 2);
    assert_eq!(params.get("user_id"), Some(&"123".to_string()));
    assert_eq!(params.get("post_id"), Some(&"456".to_string()));
}

#[test]
fn test_extract_path_params_no_params() {
    let params = extract_path_params("/health", "/health");
    
    assert_eq!(params.len(), 0);
}

#[test]
fn test_extract_path_params_special_characters() {
    let params = extract_path_params("/hello/Test-User_123", "/hello/{name}");
    
    assert_eq!(params.len(), 1);
    assert_eq!(params.get("name"), Some(&"Test-User_123".to_string()));
}

#[test]
fn test_extract_path_params_url_encoded() {
    let params = extract_path_params("/hello/John%20Doe", "/hello/{name}");
    
    assert_eq!(params.len(), 1);
    assert_eq!(params.get("name"), Some(&"John%20Doe".to_string()));
}

#[test]
fn test_extract_path_params_empty_param() {
    let params = extract_path_params("/hello/", "/hello/{name}");
    
    assert_eq!(params.len(), 1);
    assert_eq!(params.get("name"), Some(&"".to_string()));
}

#[test]
fn test_extract_path_params_mismatched_pattern() {
    // Test with more path segments than pattern
    let params = extract_path_params("/hello/world/extra", "/hello/{name}");
    
    assert_eq!(params.len(), 1);
    assert_eq!(params.get("name"), Some(&"world".to_string()));
}

#[test]
fn test_extract_path_params_fewer_segments() {
    // Test with fewer path segments than pattern
    let params = extract_path_params("/hello", "/hello/{name}");
    
    assert_eq!(params.len(), 0); // No name parameter found
}

#[test]
fn test_api_response_serialization() {
    let data = HelloData {
        greeting: "Hello, Serialization!".to_string(),
        name: Some("Serialization".to_string()),
    };
    
    let response = ApiResponse::success(data, "Serialization test");
    let json = serde_json::to_string(&response).unwrap();
    
    // Basic checks that JSON contains expected fields
    assert!(json.contains("success"));
    assert!(json.contains("true"));
    assert!(json.contains("Hello, Serialization!"));
    assert!(json.contains("Serialization test"));
}

#[test]
fn test_api_response_deserialization() {
    let json = r#"
    {
        "success": true,
        "message": "Test message",
        "data": {
            "greeting": "Hello, Test!",
            "name": "Test"
        },
        "timestamp": "2023-01-01T00:00:00Z"
    }
    "#;
    
    let response: ApiResponse<HelloData> = serde_json::from_str(json).unwrap();
    
    assert!(response.success);
    assert_eq!(response.message, "Test message");
    assert!(response.data.is_some());
    
    let data = response.data.unwrap();
    assert_eq!(data.greeting, "Hello, Test!");
    assert_eq!(data.name, Some("Test".to_string()));
}

#[test]
fn test_timestamp_format() {
    let response = ApiResponse::<()>::error("Test error");
    
    // Check that timestamp is in RFC3339 format (ISO 8601)
    assert!(response.timestamp.contains("T"));
    assert!(response.timestamp.ends_with("Z") || response.timestamp.contains("+") || response.timestamp.contains("-"));
}

/// Test edge cases and error conditions
#[test]
fn test_edge_cases() {
    // Test empty strings
    let empty_response = ApiResponse::<()>::error("");
    assert_eq!(empty_response.message, "");
    
    // Test very long messages
    let long_message = "a".repeat(1000);
    let long_response = ApiResponse::<()>::error(&long_message);
    assert_eq!(long_response.message, long_message);
}

/// Performance test for path parameter extraction
#[test]
fn test_path_param_extraction_performance() {
    let start = std::time::Instant::now();
    
    // Extract parameters many times
    for i in 0..1000 {
        let path = format!("/hello/user{}", i);
        let params = extract_path_params(&path, "/hello/{name}");
        assert_eq!(params.get("name"), Some(&format!("user{}", i)));
    }
    
    let duration = start.elapsed();
    
    // Should complete quickly (under 10ms for 1000 extractions)
    assert!(duration.as_millis() < 10, "Path extraction too slow: {:?}", duration);
}