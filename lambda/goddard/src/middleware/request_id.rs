use axum::{
    extract::Request,
    response::Response,
    middleware::Next,
};
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(skip_all)]
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    
    info!("Generated request ID: {}", request_id);
    
    // Add request ID to request extensions
    request.extensions_mut().insert(request_id.clone());
    
    // Process the request
    let mut response = next.run(request).await;
    
    // Add request ID to response headers
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );
    
    response
}