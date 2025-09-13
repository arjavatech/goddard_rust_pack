use axum::{
    extract::Request,
    response::Response,
    middleware::Next,
    http::HeaderValue,
};

pub async fn add_cors_headers(
    request: Request,
    next: Next,
) -> Response {
    // Handle OPTIONS requests directly
    if request.method().as_str() == "OPTIONS" {
        return handle_cors_preflight().await;
    }
    
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    headers.insert(
        "access-control-allow-origin",
        HeaderValue::from_static("*"),
    );
    
    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    
    headers.insert(
        "access-control-allow-headers",
        HeaderValue::from_static("Content-Type, Authorization, x-request-id"),
    );
    
    headers.insert(
        "access-control-max-age",
        HeaderValue::from_static("86400"),
    );
    
    response
}

pub async fn handle_cors_preflight() -> Response {
    Response::builder()
        .status(204)
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("access-control-allow-headers", "Content-Type, Authorization, x-request-id")
        .header("access-control-max-age", "86400")
        .body(axum::body::Body::empty())
        .unwrap()
}