pub mod connection;
pub mod schools;
pub mod users;
// Temporarily disabled modules due to schema mismatches
// pub mod children;
// pub mod classrooms;
// pub mod enrollments;
// pub mod forms;
// pub mod notifications;
// pub mod documents;
// pub mod admin;

use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub type DbPool = Arc<Pool<Postgres>>;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database query failed: {0}")]
    QueryError(#[from] sqlx::Error),

    #[error("Record not found")]
    NotFound,

    #[error("Duplicate record")]
    DuplicateRecord,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Unauthorized")]
    Unauthorized,
}