pub mod connection;
// All DB modules temporarily disabled - will be reimplemented following new API specification
// pub mod schools;
// pub mod users;
// pub mod children;
// pub mod classrooms;
// pub mod enrollments;
// pub mod forms;
// pub mod notifications;
// pub mod documents;
// pub mod admin;

use deadpool_postgres::Pool;
use std::sync::Arc;

pub type DbPool = Arc<Pool>;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database query failed: {0}")]
    QueryError(#[from] tokio_postgres::Error),

    #[error("Record not found")]
    NotFound,

    #[error("Duplicate record")]
    DuplicateRecord,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Unauthorized")]
    Unauthorized,
}