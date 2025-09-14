use axum::{
    extract::{Path, Query, Multipart},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::schema::{
    Document, DocumentType, UploadDocumentRequest,
    QueryParams, ApiResponse, PaginatedResponse, PaginationMeta, PaginationLinks
};
// use crate::db::documents;

#[derive(Deserialize)]
pub struct UpdateDocumentRequest {
    pub name: Option<String>,
    pub document_type: Option<DocumentType>,
    pub description: Option<String>,
}

pub async fn list_documents(
    Extension(_user_id): Extension<Uuid>,
    Extension(school_id): Extension<Uuid>,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedResponse<Document>>, StatusCode> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);

    // Create database connection
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost/goddard_db".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // TODO: Re-enable when database module is fixed
    let documents_list = vec![];
    let total_count = 0i64;

    let total_pages = if total_count == 0 { 1 } else { (total_count + limit as i64 - 1) / limit as i64 };

    let response = PaginatedResponse {
        data: documents_list,
        meta: PaginationMeta {
            page,
            per_page: limit,
            total: total_count as i32,
            total_pages: total_pages as i32,
        },
        links: PaginationLinks {
            self_link: format!("/documents?page={}&limit={}", page, limit),
            next: if page < total_pages as i32 { Some(format!("/documents?page={}&limit={}", page + 1, limit)) } else { None },
            prev: if page > 1 { Some(format!("/documents?page={}&limit={}", page - 1, limit)) } else { None },
            first: format!("/documents?page=1&limit={}", limit),
            last: format!("/documents?page={}&limit={}", total_pages, limit),
        },
    };

    Ok(Json(response))
}

pub async fn upload_document(
    Extension(_user_id): Extension<Uuid>,
    Extension(_school_id): Extension<Uuid>,
    _multipart: Multipart,
) -> Result<Json<ApiResponse<Document>>, StatusCode> {
    // TODO: Implement document upload to S3
    // TODO: Create document record in database
    // TODO: Process multipart form data
    tracing::info!("Document upload requested");

    Err(StatusCode::NOT_IMPLEMENTED)
}