use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard API response structure
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            message: message.to_string(),
            data: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Health check response data
#[derive(Serialize, Deserialize, Debug)]
pub struct HealthData {
    pub status: String,
    pub version: String,
    pub uptime: String,
}

/// Hello response data
#[derive(Serialize, Deserialize, Debug)]
pub struct HelloData {
    pub greeting: String,
    pub name: Option<String>,
}

/// Extract path parameters from the request
pub fn extract_path_params(path: &str, pattern: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    
    let path_parts: Vec<&str> = path.split('/').collect();
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    
    for (i, pattern_part) in pattern_parts.iter().enumerate() {
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            let param_name = &pattern_part[1..pattern_part.len()-1];
            if let Some(value) = path_parts.get(i) {
                params.insert(param_name.to_string(), value.to_string());
            }
        }
    }
    
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let data = HelloData {
            greeting: "Hello".to_string(),
            name: Some("World".to_string()),
        };
        let response = ApiResponse::success(data, "Success");
        
        assert!(response.success);
        assert_eq!(response.message, "Success");
        assert!(response.data.is_some());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("Error occurred");
        
        assert!(!response.success);
        assert_eq!(response.message, "Error occurred");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_extract_path_params() {
        let params = extract_path_params("/hello/John", "/hello/{name}");
        assert_eq!(params.get("name"), Some(&"John".to_string()));
    }
}