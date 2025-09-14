use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::schema::{Document, DocumentType, DocumentStatus, UploadDocumentRequest};
use super::DbError;

pub async fn get_documents_by_enrollment(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    enrollment_id: Uuid,
    page: i32,
    limit: i32,
) -> Result<Vec<Document>, DbError> {
    let offset = (page - 1) * limit;

    let rows = sqlx::query!(
        r#"
        SELECT
            d.id,
            d.enrollment_id,
            d.school_id,
            d.file_name,
            d.document_type,
            d.storage_path,
            d.file_size,
            d.mime_type,
            d.uploaded_at
        FROM documents d
        WHERE d.school_id = $1
            AND d.enrollment_id = $2
            AND d.is_active = true
        ORDER BY d.uploaded_at DESC
        LIMIT $3 OFFSET $4
        "#,
        school_id,
        enrollment_id,
        limit as i64,
        offset as i64
    )
    .fetch_all(pool)
    .await?;

    let documents = rows
        .into_iter()
        .map(|row| Document {
            id: row.id,
            enrollment_id: row.enrollment_id,
            school_id: row.school_id,
            document_name: row.file_name.clone(),
            document_type: parse_document_type(&row.document_type),
            file_path: row.storage_path,
            file_size: row.file_size.unwrap_or(0),
            mime_type: row.mime_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            status: DocumentStatus::Pending, // Default status since not in DB
            uploaded_at: row.uploaded_at,
            processed_at: None, // Not available in current schema
        })
        .collect();

    Ok(documents)
}

pub async fn upload_document(
    pool: &Pool<Postgres>,
    school_id: Uuid,
    enrollment_id: Uuid,
    request: UploadDocumentRequest,
    uploaded_by: Uuid,
) -> Result<Document, DbError> {
    let document_type_str = match request.document_type {
        DocumentType::Medical => "medical",
        DocumentType::Emergency => "emergency",
        DocumentType::Authorization => "authorization",
        DocumentType::Photo => "photo",
        DocumentType::Other => "other",
    };

    let row = sqlx::query!(
        r#"
        INSERT INTO documents (
            enrollment_id,
            school_id,
            file_name,
            document_type,
            storage_path,
            file_size,
            mime_type,
            uploaded_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING
            id,
            enrollment_id,
            school_id,
            file_name,
            document_type,
            storage_path,
            file_size,
            mime_type,
            uploaded_at
        "#,
        enrollment_id,
        school_id,
        request.document_name,
        document_type_str,
        request.file_path,
        request.file_size,
        request.mime_type,
        uploaded_by
    )
    .fetch_one(pool)
    .await?;

    Ok(Document {
        id: row.id,
        enrollment_id: row.enrollment_id,
        school_id: row.school_id,
        document_name: row.file_name,
        document_type: request.document_type,
        file_path: row.storage_path,
        file_size: row.file_size.unwrap_or(0),
        mime_type: row.mime_type.unwrap_or_else(|| "application/octet-stream".to_string()),
        status: DocumentStatus::Pending, // Default since not in DB
        uploaded_at: row.uploaded_at,
        processed_at: None, // Not available in current schema
    })
}

// TODO: Implement document status update when schema supports it
// pub async fn update_document_status(
//     pool: &Pool<Postgres>,
//     document_id: Uuid,
//     school_id: Uuid,
//     status: DocumentStatus,
// ) -> Result<(), DbError> {
//     // Current schema doesn't support status field
//     Ok(())
// }

fn parse_document_type(doc_type: &str) -> DocumentType {
    match doc_type {
        "medical" => DocumentType::Medical,
        "emergency" => DocumentType::Emergency,
        "authorization" => DocumentType::Authorization,
        "photo" => DocumentType::Photo,
        "other" => DocumentType::Other,
        _ => DocumentType::Other,
    }
}

fn parse_document_status(status: &str) -> DocumentStatus {
    match status {
        "pending" => DocumentStatus::Pending,
        "approved" => DocumentStatus::Approved,
        "rejected" => DocumentStatus::Rejected,
        "processing" => DocumentStatus::Processing,
        _ => DocumentStatus::Pending,
    }
}