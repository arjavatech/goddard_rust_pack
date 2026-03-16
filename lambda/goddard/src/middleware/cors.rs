use axum::{
    extract::Request,
    response::Response,
    middleware::Next,
    http::HeaderValue,
};

fn is_allowed_origin(origin: &str) -> bool {
    // Allow localhost 5000 series (5000-5999) and 3000 series (3000-3999)
    if origin.starts_with("http://localhost:") {
        if let Some(port_str) = origin.strip_prefix("http://localhost:") {
            if let Ok(port) = port_str.parse::<u16>() {
                if (3000..=3999).contains(&port) || (5000..=5999).contains(&port) {
                    return true;
                }
            }
        }
    }

    // Allow HTTPS localhost 5000 series and 3000 series
    if origin.starts_with("https://localhost:") {
        if let Some(port_str) = origin.strip_prefix("https://localhost:") {
            if let Ok(port) = port_str.parse::<u16>() {
                if (3000..=3999).contains(&port) || (5000..=5999).contains(&port) {
                    return true;
                }
            }
        }
    }

    // Allow goddardschool.org domains
    if origin.contains("goddardschool.org") {
        return true;
    }

    // Allow 127.0.0.1 5000 series as well
    if origin.starts_with("http://127.0.0.1:5") || origin.starts_with("https://127.0.0.1:5") {
        if let Some(port_start) = origin.rfind(':') {
            if let Ok(port) = origin[port_start + 1..].parse::<u16>() {
                if (5000..=5999).contains(&port) {
                    return true;
                }
            }
        }
    }

    // Allow 127.0.0.1 3000 series as well
    if origin.starts_with("http://127.0.0.1:3") || origin.starts_with("https://127.0.0.1:3") {
        if let Some(port_start) = origin.rfind(':') {
            if let Ok(port) = origin[port_start + 1..].parse::<u16>() {
                if (3000..=3999).contains(&port) {
                    return true;
                }
            }
        }
    }

    false
}

pub async fn add_cors_headers(
    request: Request,
    next: Next,
) -> Response {
    // Get the Origin header from the request
    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| String::new());

    // Handle OPTIONS requests directly
    if request.method().as_str() == "OPTIONS" {
        return handle_cors_preflight(&origin).await;
    }

    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Set allowed origin based on the request origin
    // For development, be more permissive
    let allowed_origin = if origin.is_empty() {
        HeaderValue::from_static("*")
    } else if is_allowed_origin(&origin) {
        HeaderValue::from_str(&origin).unwrap_or(HeaderValue::from_static("*"))
    } else {
        // For Lambda/production, still allow all origins for now (can be restricted later)
        HeaderValue::from_static("*")
    };

    headers.insert(
        "access-control-allow-origin",
        allowed_origin,
    );

    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS, PATCH"),
    );

    headers.insert(
        "access-control-allow-headers",
        HeaderValue::from_static("Content-Type, Authorization, x-request-id, x-school-id, x-api-key"),
    );

    headers.insert(
        "access-control-allow-credentials",
        HeaderValue::from_static("true"),
    );

    headers.insert(
        "access-control-max-age",
        HeaderValue::from_static("86400"),
    );

    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_static("Content-Disposition"),
    );

    response
}

pub async fn handle_cors_preflight(origin: &str) -> Response {
    let allowed_origin = if is_allowed_origin(origin) {
        origin
    } else {
        "*"
    };

    Response::builder()
        .status(204)
        .header("access-control-allow-origin", allowed_origin)
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS, PATCH")
        .header("access-control-allow-headers", "Content-Type, Authorization, x-request-id, x-school-id, x-api-key")
        .header("access-control-allow-credentials", "true")
        .header("access-control-max-age", "86400")
        .header("access-control-expose-headers", "Content-Disposition")
        .body(axum::body::Body::empty())
        .unwrap()
}