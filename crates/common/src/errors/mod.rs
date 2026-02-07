//! Error types for PaperForge services
//! 
//! Provides a comprehensive error handling system with:
//! - Distinct error types for different failure modes
//! - HTTP status code mapping
//! - Structured error responses
//! - Error codes for client handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Result type alias using AppError
pub type Result<T> = std::result::Result<T, AppError>;

/// Error codes for machine-readable error identification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // Validation errors (1xxx)
    ValidationError,
    MissingField,
    InvalidFormat,
    PayloadTooLarge,
    
    // Authentication errors (2xxx)
    Unauthorized,
    InvalidApiKey,
    ExpiredToken,
    
    // Authorization errors (3xxx)  
    Forbidden,
    InsufficientPermissions,
    TenantMismatch,
    
    // Resource errors (4xxx)
    NotFound,
    PaperNotFound,
    ChunkNotFound,
    JobNotFound,
    SessionNotFound,
    
    // Conflict errors (5xxx)
    Conflict,
    DuplicatePaper,
    DuplicateIdempotencyKey,
    
    // Rate limiting (6xxx)
    RateLimited,
    QuotaExceeded,
    
    // Database errors (7xxx)
    DatabaseError,
    ConnectionError,
    TransactionError,
    
    // External service errors (8xxx)
    UpstreamError,
    EmbeddingError,
    EmbeddingTimeout,
    CircuitBreakerOpen,
    QueueError,
    CacheError,
    
    // Internal errors (9xxx)
    InternalError,
    ConfigurationError,
    SerializationError,
    
    // Service unavailable
    ServiceUnavailable,
}

impl ErrorCode {
    /// Get the numeric code for this error
    pub fn as_code(&self) -> u16 {
        match self {
            // Validation (1xxx)
            ErrorCode::ValidationError => 1001,
            ErrorCode::MissingField => 1002,
            ErrorCode::InvalidFormat => 1003,
            ErrorCode::PayloadTooLarge => 1004,
            
            // Auth (2xxx)
            ErrorCode::Unauthorized => 2001,
            ErrorCode::InvalidApiKey => 2002,
            ErrorCode::ExpiredToken => 2003,
            
            // Authz (3xxx)
            ErrorCode::Forbidden => 3001,
            ErrorCode::InsufficientPermissions => 3002,
            ErrorCode::TenantMismatch => 3003,
            
            // Resources (4xxx)
            ErrorCode::NotFound => 4001,
            ErrorCode::PaperNotFound => 4002,
            ErrorCode::ChunkNotFound => 4003,
            ErrorCode::JobNotFound => 4004,
            ErrorCode::SessionNotFound => 4005,
            
            // Conflicts (5xxx)
            ErrorCode::Conflict => 5001,
            ErrorCode::DuplicatePaper => 5002,
            ErrorCode::DuplicateIdempotencyKey => 5003,
            
            // Rate limits (6xxx)
            ErrorCode::RateLimited => 6001,
            ErrorCode::QuotaExceeded => 6002,
            
            // Database (7xxx)
            ErrorCode::DatabaseError => 7001,
            ErrorCode::ConnectionError => 7002,
            ErrorCode::TransactionError => 7003,
            
            // External (8xxx)
            ErrorCode::UpstreamError => 8001,
            ErrorCode::EmbeddingError => 8002,
            ErrorCode::EmbeddingTimeout => 8003,
            ErrorCode::CircuitBreakerOpen => 8004,
            ErrorCode::QueueError => 8005,
            ErrorCode::CacheError => 8006,
            
            // Internal (9xxx)
            ErrorCode::InternalError => 9001,
            ErrorCode::ConfigurationError => 9002,
            ErrorCode::SerializationError => 9003,
            
            ErrorCode::ServiceUnavailable => 9999,
        }
    }
}

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    // Validation errors
    #[error("Validation failed: {message}")]
    Validation { 
        message: String, 
        field: Option<String> 
    },
    
    #[error("Required field missing: {field}")]
    MissingField { field: String },
    
    #[error("Invalid format: {message}")]
    InvalidFormat { message: String },
    
    #[error("Payload too large: {size} bytes exceeds limit of {limit} bytes")]
    PayloadTooLarge { size: usize, limit: usize },
    
    // Authentication errors
    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Token expired")]
    ExpiredToken,
    
    // Authorization errors
    #[error("Forbidden: {message}")]
    Forbidden { message: String },
    
    #[error("Tenant mismatch")]
    TenantMismatch,
    
    // Resource errors
    #[error("Resource not found: {resource_type} with id {id}")]
    NotFound { resource_type: String, id: String },
    
    #[error("Paper not found: {id}")]
    PaperNotFound { id: String },
    
    #[error("Job not found: {id}")]
    JobNotFound { id: String },
    
    #[error("Session not found: {id}")]
    SessionNotFound { id: String },
    
    // Conflict errors
    #[error("Duplicate resource: {message}")]
    Duplicate { message: String },
    
    #[error("Duplicate idempotency key: {key}")]
    DuplicateIdempotencyKey { key: String },
    
    // Rate limiting
    #[error("Rate limit exceeded: {limit} requests per second")]
    RateLimited { limit: u32 },
    
    // Database errors
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("Database connection error: {message}")]
    DatabaseConnection { message: String },
    
    // External service errors
    #[error("Embedding service error: {message}")]
    EmbeddingError { message: String },
    
    #[error("Embedding timeout after {timeout_ms}ms")]
    EmbeddingTimeout { timeout_ms: u64 },
    
    #[error("Circuit breaker open for service: {service}")]
    CircuitBreakerOpen { service: String },
    
    #[error("Queue error: {message}")]
    QueueError { message: String },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
    
    // Internal errors
    #[error("Internal server error: {message}")]
    Internal { message: String },
    
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },
    
    // Generic
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl AppError {
    /// Get the error code for this error
    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::Validation { .. } => ErrorCode::ValidationError,
            AppError::MissingField { .. } => ErrorCode::MissingField,
            AppError::InvalidFormat { .. } => ErrorCode::InvalidFormat,
            AppError::PayloadTooLarge { .. } => ErrorCode::PayloadTooLarge,
            AppError::Unauthorized { .. } => ErrorCode::Unauthorized,
            AppError::InvalidApiKey => ErrorCode::InvalidApiKey,
            AppError::ExpiredToken => ErrorCode::ExpiredToken,
            AppError::Forbidden { .. } => ErrorCode::Forbidden,
            AppError::TenantMismatch => ErrorCode::TenantMismatch,
            AppError::NotFound { .. } => ErrorCode::NotFound,
            AppError::PaperNotFound { .. } => ErrorCode::PaperNotFound,
            AppError::JobNotFound { .. } => ErrorCode::JobNotFound,
            AppError::SessionNotFound { .. } => ErrorCode::SessionNotFound,
            AppError::Duplicate { .. } => ErrorCode::Conflict,
            AppError::DuplicateIdempotencyKey { .. } => ErrorCode::DuplicateIdempotencyKey,
            AppError::RateLimited { .. } => ErrorCode::RateLimited,
            AppError::Database(_) => ErrorCode::DatabaseError,
            AppError::DatabaseConnection { .. } => ErrorCode::ConnectionError,
            AppError::EmbeddingError { .. } => ErrorCode::EmbeddingError,
            AppError::EmbeddingTimeout { .. } => ErrorCode::EmbeddingTimeout,
            AppError::CircuitBreakerOpen { .. } => ErrorCode::CircuitBreakerOpen,
            AppError::QueueError { .. } => ErrorCode::QueueError,
            AppError::CacheError { .. } => ErrorCode::CacheError,
            AppError::HttpClient(_) => ErrorCode::UpstreamError,
            AppError::Internal { .. } => ErrorCode::InternalError,
            AppError::Configuration { .. } => ErrorCode::ConfigurationError,
            AppError::Serialization(_) => ErrorCode::SerializationError,
            AppError::ServiceUnavailable { .. } => ErrorCode::ServiceUnavailable,
            AppError::Other(_) => ErrorCode::InternalError,
        }
    }
    
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 400 Bad Request
            AppError::Validation { .. } |
            AppError::MissingField { .. } |
            AppError::InvalidFormat { .. } => StatusCode::BAD_REQUEST,
            
            // 401 Unauthorized
            AppError::Unauthorized { .. } |
            AppError::InvalidApiKey |
            AppError::ExpiredToken => StatusCode::UNAUTHORIZED,
            
            // 403 Forbidden
            AppError::Forbidden { .. } |
            AppError::TenantMismatch => StatusCode::FORBIDDEN,
            
            // 404 Not Found
            AppError::NotFound { .. } |
            AppError::PaperNotFound { .. } |
            AppError::JobNotFound { .. } |
            AppError::SessionNotFound { .. } => StatusCode::NOT_FOUND,
            
            // 409 Conflict
            AppError::Duplicate { .. } |
            AppError::DuplicateIdempotencyKey { .. } => StatusCode::CONFLICT,
            
            // 413 Payload Too Large
            AppError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            
            // 429 Too Many Requests
            AppError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            
            // 500 Internal Server Error
            AppError::Database(_) |
            AppError::DatabaseConnection { .. } |
            AppError::Internal { .. } |
            AppError::Configuration { .. } |
            AppError::Serialization(_) |
            AppError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
            
            // 502 Bad Gateway
            AppError::EmbeddingError { .. } |
            AppError::EmbeddingTimeout { .. } |
            AppError::HttpClient(_) => StatusCode::BAD_GATEWAY,
            
            // 503 Service Unavailable
            AppError::CircuitBreakerOpen { .. } |
            AppError::QueueError { .. } |
            AppError::CacheError { .. } |
            AppError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    
    /// Check if this error should be logged at error level
    pub fn is_server_error(&self) -> bool {
        self.status_code().is_server_error()
    }
    
    /// Check if this error is a client error  
    pub fn is_client_error(&self) -> bool {
        self.status_code().is_client_error()
    }
}

/// Structured error response for API
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.code();
        let message = self.to_string();
        
        // Log based on severity
        if self.is_server_error() {
            tracing::error!(
                error = %message,
                code = ?code,
                status = status.as_u16(),
                "Server error"
            );
        } else if self.is_client_error() {
            tracing::warn!(
                error = %message,
                code = ?code,
                status = status.as_u16(),
                "Client error"
            );
        }
        
        let body = ErrorResponse {
            error: ErrorDetails {
                code,
                message,
                details: None,
                request_id: None, // Should be filled by middleware
            },
        };
        
        (status, Json(body)).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal { 
            message: err.to_string() 
        }
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::CacheError { 
            message: err.to_string() 
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_code_mapping() {
        let err = AppError::PaperNotFound { id: "test".into() };
        assert_eq!(err.code(), ErrorCode::PaperNotFound);
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }
    
    #[test]
    fn test_validation_error() {
        let err = AppError::Validation { 
            message: "Invalid title".into(),
            field: Some("title".into()),
        };
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert!(!err.is_server_error());
        assert!(err.is_client_error());
    }
    
    #[test]
    fn test_server_error() {
        let err = AppError::Internal { 
            message: "Something went wrong".into() 
        };
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(err.is_server_error());
    }
}
