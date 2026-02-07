//! PaperForge Common Library
//! 
//! Shared code for all PaperForge microservices including:
//! - Database models and repository patterns
//! - Embedding client abstraction
//! - Error types and handling
//! - Configuration management
//! - Authentication utilities
//! - Metrics and observability
//! - gRPC protocol definitions

pub mod auth;
pub mod config;
pub mod context;
pub mod db;
pub mod embeddings;
pub mod errors;
pub mod metrics;
pub mod queue;
pub mod cache;

// gRPC proto definitions (generated at build time)
pub mod proto {
    // Include generated proto code
    pub mod search {
        tonic::include_proto!("paperforge.search.v2");
    }
    pub mod ingestion {
        tonic::include_proto!("paperforge.ingestion.v2");
    }
    pub mod context {
        tonic::include_proto!("paperforge.context.v2");
    }
    pub mod embedding {
        tonic::include_proto!("paperforge.embedding.v2");
    }
}

// Re-export commonly used types
pub use errors::{AppError, Result};
pub use config::AppConfig;
pub use db::{Repository, ChunkResult};
pub use embeddings::Embedder;

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default embedding model
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-ada-002";

/// Default embedding dimension
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 768;
