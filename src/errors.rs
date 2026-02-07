use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("Embedding Service Error: {0}")]
    EmbeddingError(String),

    #[error("Invalid Input: {0}")]
    ValidationError(String),

    #[error("Not Found")]
    NotFound,

    #[error("Internal Server Error: {0}")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            AppError::ConfigError(err) => {
                tracing::error!("Config error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error")
            }
            AppError::EmbeddingError(msg) => {
                tracing::error!("Embedding error: {}", msg);
                (StatusCode::BAD_GATEWAY, msg.as_str())
            }
            AppError::ValidationError(msg) => {
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::NotFound => {
                (StatusCode::NOT_FOUND, "Resource not found")
            }
            AppError::InternalError(err) => {
                tracing::error!("Internal error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": error_message,
                // In dev mode, we could include the debug info
                "details": if cfg!(debug_assertions) { Some(format!("{:?}", self)) } else { None }
            }
        }));

        (status, body).into_response()
    }
}
