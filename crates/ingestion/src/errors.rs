//! Ingestion service error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IngestionError {
    #[error("PDF parse error for {path}: {message}")]
    PdfParseError { path: String, message: String },

    #[error("Chunking error: {0}")]
    ChunkingError(String),

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<paperforge_common::errors::AppError> for IngestionError {
    fn from(e: paperforge_common::errors::AppError) -> Self {
        IngestionError::DatabaseError(e.to_string())
    }
}
