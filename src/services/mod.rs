use crate::db::Repository;
use crate::embeddings::Embedder;
use crate::services::ingest::IngestService;
use crate::services::search::SearchService;
use std::sync::Arc;

pub mod ingest;
pub mod search;

// A container for all services to be injected into routes
#[derive(Clone)]
pub struct AppState {
    pub ingest_service: Arc<IngestService>,
    pub search_service: Arc<SearchService>,
}

impl AppState {
    pub fn new(repo: Repository, embedder: Arc<dyn Embedder>) -> Self {
        // Repository is cheap to clone (Arc<DatabaseConnection> inside)
        // embeddings is Arc<dyn Embedder>
        
        Self {
            ingest_service: Arc::new(IngestService::new(repo.clone(), embedder.clone())),
            search_service: Arc::new(SearchService::new(repo, embedder)),
        }
    }
}
