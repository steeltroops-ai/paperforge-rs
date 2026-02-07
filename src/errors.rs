use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Unique error codes for client identification
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    // Database errors (1xxx)
    DatabaseConnection = 1001,
    DatabaseQuery = 1002,
    DatabaseTransaction = 1003,
    
    // Validation errors (2xxx)
    ValidationFailed = 2001,
    PayloadTooLarge = 2002,
    InvalidFormat = 2003,
    MissingField = 2004,
    
    // Authentication errors (3xxx)
    Unauthorized = 3001,
    InvalidApiKey = 3002,
    ExpiredToken = 3003,
    
    // Rate limiting errors (4xxx)
    RateLimitExceeded = 4001,
    
    // External service errors (5xxx)
    EmbeddingServiceUnavailable = 5001,
    EmbeddingServiceTimeout = 5002,
    EmbeddingServiceError = 5003,
    CircuitBreakerOpen = 5004,
    
    // Resource errors (6xxx)
    NotFound = 6001,
    AlreadyExists = 6002,
    
    // Internal errors (9xxx)
    InternalError = 9001,
    ConfigurationError = 9002,
    SerializationError = 9003,
}

impl ErrorCode {
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }
}

/// Production-grade error types with context
#[derive(Error, Debug)]
pub enum AppError {
    // Database errors
    #[error("Database connection error: {0}")]
    DatabaseConnectionError(String),
    
    #[error("Database query error: {0}")]
    DatabaseQueryError(#[from] sea_orm::DbErr),
    
    #[error("Database transaction failed: {0}")]
    DatabaseTransactionError(String),
    
    // Validation errors
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("Payload too large: {size} bytes exceeds limit of {limit} bytes")]
    PayloadTooLarge { size: usize, limit: usize },
    
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    // Authentication errors
    #[error("Authentication required")]
    Unauthorized,
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Token expired")]
    ExpiredToken,
    
    // Rate limiting
    #[error("Rate limit exceeded. Retry after {retry_after_secs} seconds")]
    RateLimitExceeded { retry_after_secs: u64 },
    
    // External service errors
    #[error("Embedding service unavailable: {0}")]
    EmbeddingServiceUnavailable(String),
    
    #[error("Embedding service timeout after {timeout_secs}s")]
    EmbeddingServiceTimeout { timeout_secs: u64 },
    
    #[error("Embedding service error: {0}")]
    EmbeddingError(String),
    
    #[error("Circuit breaker open for {service}")]
    CircuitBreakerOpen { service: String },
    
    // Resource errors
    #[error("Resource not found: {resource_type} with id {resource_id}")]
    NotFound { resource_type: String, resource_id: String },
    
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),
    
    // Internal errors
    #[error("Internal server error: {0}")]
    InternalError(#[from] anyhow::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl AppError {
    /// Get the error code for this error type
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::DatabaseConnectionError(_) => ErrorCode::DatabaseConnection,
            Self::DatabaseQueryError(_) => ErrorCode::DatabaseQuery,
            Self::DatabaseTransactionError(_) => ErrorCode::DatabaseTransaction,
            Self::ValidationError(_) => ErrorCode::ValidationFailed,
            Self::PayloadTooLarge { .. } => ErrorCode::PayloadTooLarge,
            Self::InvalidFormat(_) => ErrorCode::InvalidFormat,
            Self::MissingField(_) => ErrorCode::MissingField,
            Self::Unauthorized => ErrorCode::Unauthorized,
            Self::InvalidApiKey => ErrorCode::InvalidApiKey,
            Self::ExpiredToken => ErrorCode::ExpiredToken,
            Self::RateLimitExceeded { .. } => ErrorCode::RateLimitExceeded,
            Self::EmbeddingServiceUnavailable(_) => ErrorCode::EmbeddingServiceUnavailable,
            Self::EmbeddingServiceTimeout { .. } => ErrorCode::EmbeddingServiceTimeout,
            Self::EmbeddingError(_) => ErrorCode::EmbeddingServiceError,
            Self::CircuitBreakerOpen { .. } => ErrorCode::CircuitBreakerOpen,
            Self::NotFound { .. } => ErrorCode::NotFound,
            Self::AlreadyExists(_) => ErrorCode::AlreadyExists,
            Self::InternalError(_) => ErrorCode::InternalError,
            Self::ConfigError(_) => ErrorCode::ConfigurationError,
            Self::SerializationError(_) => ErrorCode::SerializationError,
        }
    }
    
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseConnectionError(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::DatabaseQueryError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DatabaseTransactionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            Self::InvalidFormat(_) => StatusCode::BAD_REQUEST,
            Self::MissingField(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::InvalidApiKey => StatusCode::UNAUTHORIZED,
            Self::ExpiredToken => StatusCode::UNAUTHORIZED,
            Self::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::EmbeddingServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::EmbeddingServiceTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            Self::EmbeddingError(_) => StatusCode::BAD_GATEWAY,
            Self::CircuitBreakerOpen { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::AlreadyExists(_) => StatusCode::CONFLICT,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();
        
        // Log based on severity
        match &self {
            AppError::ValidationError(_) | 
            AppError::PayloadTooLarge { .. } |
            AppError::InvalidFormat(_) |
            AppError::MissingField(_) |
            AppError::NotFound { .. } => {
                tracing::debug!(error_code = error_code.as_u16(), %message, "Client error");
            }
            AppError::Unauthorized | 
            AppError::InvalidApiKey |
            AppError::ExpiredToken |
            AppError::RateLimitExceeded { .. } => {
                tracing::info!(error_code = error_code.as_u16(), %message, "Auth/rate error");
            }
            _ => {
                tracing::error!(error_code = error_code.as_u16(), %message, error = ?self, "Server error");
            }
        };

        let body = Json(json!({
            "error": {
                "code": error_code.as_u16(),
                "status": status.as_u16(),
                "message": message,
                "details": if cfg!(debug_assertions) { 
                    Some(format!("{:?}", self)) 
                } else { 
                    None 
                }
            }
        }));

        // Add Retry-After header for rate limiting
        let mut response = (status, body).into_response();
        if let AppError::RateLimitExceeded { retry_after_secs } = &self {
            response.headers_mut().insert(
                "Retry-After",
                retry_after_secs.to_string().parse().unwrap(),
            );
        }
        
        response
    }
}

/// Helper macro for creating NotFound errors
#[macro_export]
macro_rules! not_found {
    ($resource_type:expr, $resource_id:expr) => {
        AppError::NotFound {
            resource_type: $resource_type.to_string(),
            resource_id: $resource_id.to_string(),
        }
    };
}
