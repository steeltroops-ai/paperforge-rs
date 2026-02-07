//! Search service for semantic paper retrieval
//!
//! Provides hybrid search combining vector similarity and BM25 text search
//! using Reciprocal Rank Fusion (RRF) for optimal result ranking.

use crate::db::Repository;
use crate::db::repository::ChunkResult;
use crate::embeddings::Embedder;
use crate::errors::AppError;
use uuid::Uuid;
use serde::Serialize;

use std::sync::Arc;
use std::time::Instant;

pub struct SearchService {
    repo: Repository,
    embedder: Arc<dyn Embedder>,
}

/// Search result returned to API clients
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub chunk_id: Uuid,
    pub paper_id: Uuid,
    pub content: String,
    pub similarity_score: f64,
    pub token_count: i32,
    /// Embedding model used for this chunk
    pub embedding_model: String,
    /// Embedding version
    pub embedding_version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_title: Option<String>,
}

/// Search mode for query execution
#[derive(Debug, Clone, Copy, Default)]
pub enum SearchMode {
    /// Pure vector similarity search
    Vector,
    /// Hybrid: vector + BM25 text search with RRF
    #[default]
    Hybrid,
}

impl SearchService {
    pub fn new(repo: Repository, embedder: Arc<dyn Embedder>) -> Self {
        Self { repo, embedder }
    }

    /// Execute a semantic search query
    /// 
    /// # Arguments
    /// * `query_text` - The search query
    /// * `limit` - Maximum number of results (capped at 50)
    /// * `hybrid` - Whether to use hybrid search (vector + BM25)
    /// 
    /// # Returns
    /// Ranked list of matching chunks with similarity scores
    pub async fn query(
        &self, 
        query_text: String, 
        limit: u64, 
        hybrid: bool
    ) -> Result<Vec<SearchResult>, AppError> {
        let start = Instant::now();
        
        // Cap limit to prevent abuse
        let limit = limit.min(50);
        
        // 1. Generate query embedding
        let embedding_start = Instant::now();
        let embedding = self.embedder.embed_query(&query_text).await?;
        let embedding_duration = embedding_start.elapsed();
        
        metrics::histogram!("paperforge_query_embedding_duration_seconds")
            .record(embedding_duration.as_secs_f64());

        // 2. Search DB (Hybrid or Pure Vector)
        let results: Vec<(ChunkResult, f64)> = if hybrid {
            self.repo.search_hybrid(query_text.clone(), embedding, limit).await?
        } else {
            self.repo.search_similar_chunks(embedding, limit).await?
        };

        // 3. Transform to SearchResult with proper score normalization
        let mapped_results: Vec<SearchResult> = results
            .into_iter()
            .map(|(chunk, score)| {
                // For hybrid search, score is already normalized
                // For vector search, we convert distance to similarity
                let similarity_score = if hybrid {
                    score.clamp(0.0, 1.0)
                } else {
                    (1.0 - score).clamp(0.0, 1.0)
                };

                SearchResult {
                    chunk_id: chunk.id,
                    paper_id: chunk.paper_id,
                    content: chunk.content,
                    similarity_score,
                    token_count: chunk.token_count,
                    embedding_model: chunk.embedding_model,
                    embedding_version: chunk.embedding_version,
                    paper_title: None, // Could be populated with a join
                }
            })
            .collect();

        // 4. Record metrics
        let total_duration = start.elapsed();
        metrics::counter!("paperforge_search_ops_total").increment(1);
        metrics::histogram!("paperforge_search_duration_seconds")
            .record(total_duration.as_secs_f64());
        metrics::histogram!("paperforge_search_results_count")
            .record(mapped_results.len() as f64);

        tracing::debug!(
            query_len = query_text.len(),
            results = mapped_results.len(),
            hybrid = hybrid,
            duration_ms = total_duration.as_millis(),
            "Search completed"
        );

        Ok(mapped_results)
    }
}
