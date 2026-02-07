//! Service layer for PaperForge-rs
//!
//! Contains business logic services for ingestion and search.
//! Services are thread-safe and designed for shared use via Arc.

use crate::db::Repository;
use crate::embeddings::Embedder;
use crate::services::ingest::IngestService;
use crate::services::search::SearchService;
use std::sync::Arc;

pub mod ingest;
pub mod search;

/// Application state container for dependency injection
/// 
/// Contains all services and shared resources needed by route handlers.
/// Implements Clone cheaply via Arc wrapping.
#[derive(Clone)]
pub struct AppState {
    /// Paper ingestion service
    pub ingest_service: Arc<IngestService>,
    /// Semantic search service  
    pub search_service: Arc<SearchService>,
    /// Direct repository access (for idempotency checks in routes)
    pub repo: Repository,
}

impl AppState {
    /// Create new application state with initialized services
    /// 
    /// # Arguments
    /// * `repo` - Database repository (cheap to clone via internal connection pool)  
    /// * `embedder` - Embedding service implementation
    pub fn new(repo: Repository, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            ingest_service: Arc::new(IngestService::new(repo.clone(), embedder.clone())),
            search_service: Arc::new(SearchService::new(repo.clone(), embedder)),
            repo,
        }
    }
}
