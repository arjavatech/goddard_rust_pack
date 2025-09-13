use axum::{
    extract::Request,
    response::Response,
    middleware::Next,
    http::Uri,
};
use tracing::{info, debug, warn};

// Common API Gateway stage names
const COMMON_STAGES: &[&str] = &[
    "prod", "production", "live",
    "dev", "development", 
    "staging", "stage", "stg",
    "test", "testing", "qa",
    "v1", "v2", "v3", "v4", "v5",
    "beta", "alpha"
];

pub async fn strip_stage_path_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let original_uri = request.uri();
    let original_path = original_uri.path();
    
    // Try to detect and strip stage prefix
    if let Some(cleaned_path) = detect_and_strip_stage(original_path) {
        debug!("Stage path detected: '{}' -> '{}'", original_path, cleaned_path);
        
        // Build new URI with cleaned path
        match build_new_uri(original_uri, &cleaned_path) {
            Ok(new_uri) => {
                info!("Rewriting URI from '{}' to '{}'", original_uri, new_uri);
                
                // Create new request with updated URI
                let (mut parts, body) = request.into_parts();
                parts.uri = new_uri;
                request = Request::from_parts(parts, body);
            }
            Err(e) => {
                warn!("Failed to rewrite URI: {}, using original path: {}", e, original_path);
            }
        }
    } else {
        debug!("No stage prefix detected in path: {}", original_path);
    }
    
    next.run(request).await
}

pub fn detect_and_strip_stage(path: &str) -> Option<String> {
    // Handle root path
    if path == "/" {
        return None;
    }
    
    // Split path into segments
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.is_empty() || segments[0].is_empty() {
        return None;
    }
    
    let first_segment = segments[0].to_lowercase();
    
    // Check against common stage names
    if COMMON_STAGES.contains(&first_segment.as_str()) {
        return Some(build_cleaned_path(&segments[1..]));
    }
    
    // Pattern matching for stage-like segments
    if is_stage_pattern(&first_segment) {
        return Some(build_cleaned_path(&segments[1..]));
    }
    
    None
}

fn is_stage_pattern(segment: &str) -> bool {
    // Check for version patterns (v1, v2, etc.)
    if segment.starts_with('v') && segment.len() <= 3 {
        if segment[1..].parse::<u32>().is_ok() {
            return true;
        }
    }
    
    // Check for environment-like patterns (short, alphanumeric)
    if segment.len() <= 8 && segment.chars().all(|c| c.is_alphanumeric()) {
        // Avoid false positives for common path segments
        let false_positives = &[
            "api", "app", "web", "mobile", "admin", "public", "static",
            "users", "user", "products", "product", "orders", "order", 
            "items", "item", "data", "content", "assets", "files",
            "auth", "login", "logout", "register", "profile", "account",
            "search", "browse", "category", "categories", "tags", "blog",
            "docs", "help", "about", "contact", "support", "faq",
            "version", "versions", "config", "settings", "preferences",
            "hello", "health", "status", "ping", "echo", "home", "index",
            "service", "services", "resource", "resources", "endpoint"
        ];
        if false_positives.contains(&segment) {
            return false;
        }
        return true;
    }
    
    false
}

fn build_cleaned_path(segments: &[&str]) -> String {
    if segments.is_empty() || (segments.len() == 1 && segments[0].is_empty()) {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn build_new_uri(original: &Uri, new_path: &str) -> Result<Uri, Box<dyn std::error::Error + Send + Sync>> {
    let mut uri_parts = original.clone().into_parts();
    
    // Build new path and query - only update the path portion, keep the same scheme and authority
    let new_path_and_query = if let Some(query) = original.query() {
        format!("{}?{}", new_path, query)
    } else {
        new_path.to_string()
    };
    
    uri_parts.path_and_query = Some(new_path_and_query.parse()?);
    
    Ok(Uri::from_parts(uri_parts)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_common_stages() {
        assert_eq!(detect_and_strip_stage("/prod/hello/world"), Some("/hello/world".to_string()));
        assert_eq!(detect_and_strip_stage("/dev/api/users"), Some("/api/users".to_string()));
        assert_eq!(detect_and_strip_stage("/staging/health"), Some("/health".to_string()));
        assert_eq!(detect_and_strip_stage("/v1/users"), Some("/users".to_string()));
    }

    #[test]
    fn test_no_false_positives() {
        assert_eq!(detect_and_strip_stage("/users/profile"), None);
        assert_eq!(detect_and_strip_stage("/products/list"), None);
        assert_eq!(detect_and_strip_stage("/api/health"), None);
    }

    #[test]
    fn test_root_and_edge_cases() {
        assert_eq!(detect_and_strip_stage("/"), None);
        assert_eq!(detect_and_strip_stage("/prod/"), Some("/".to_string()));
        assert_eq!(detect_and_strip_stage("/prod"), Some("/".to_string()));
    }

    #[test]
    fn test_version_patterns() {
        assert_eq!(detect_and_strip_stage("/v1/api"), Some("/api".to_string()));
        assert_eq!(detect_and_strip_stage("/v12/users"), Some("/users".to_string()));
        // Should not match non-version patterns
        assert_eq!(detect_and_strip_stage("/version/api"), None);
    }

    #[test]
    fn test_build_cleaned_path() {
        assert_eq!(build_cleaned_path(&["hello", "world"]), "/hello/world");
        assert_eq!(build_cleaned_path(&[]), "/");
        assert_eq!(build_cleaned_path(&[""]), "/");
    }
}