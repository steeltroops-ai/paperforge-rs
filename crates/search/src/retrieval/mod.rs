//! Multi-modal retrieval system
//!
//! Provides three retrieval modes:
//! - Vector search (semantic similarity via embeddings)
//! - BM25 search (lexical matching)
//! - Hybrid search (RRF fusion of vector + BM25)

mod vector;
mod bm25;
mod hybrid;
mod fusion;

pub use vector::VectorRetriever;
pub use bm25::BM25Retriever;
pub use hybrid::HybridRetriever;
pub use fusion::{RRFusion, FusionResult};

use paperforge_common::errors::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Retrieved chunk with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    /// Chunk ID
    pub chunk_id: Uuid,
    
    /// Paper ID this chunk belongs to
    pub paper_id: Uuid,
    
    /// Paper title
    pub paper_title: String,
    
    /// Chunk content
    pub content: String,
    
    /// Chunk index within paper
    pub chunk_index: i32,
    
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
    
    /// Retrieval mode used
    pub retrieval_mode: RetrievalMode,
}

/// Retrieval mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    /// Vector similarity search
    Vector,
    /// BM25 lexical search
    BM25,
    /// Combined hybrid search
    Hybrid,
}

/// Search request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Tenant ID for isolation
    pub tenant_id: Uuid,
    
    /// Query text
    pub query: String,
    
    /// Query embedding (for vector search)
    pub query_embedding: Option<Vec<f32>>,
    
    /// Retrieval mode
    pub mode: RetrievalMode,
    
    /// Maximum results to return
    pub limit: usize,
    
    /// Minimum score threshold
    pub min_score: Option<f32>,
    
    /// Filter by paper IDs (optional)
    pub paper_ids: Option<Vec<Uuid>>,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            tenant_id: Uuid::nil(),
            query: String::new(),
            query_embedding: None,
            mode: RetrievalMode::Hybrid,
            limit: 10,
            min_score: Some(0.3),
            paper_ids: None,
        }
    }
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Retrieved chunks
    pub chunks: Vec<RetrievedChunk>,
    
    /// Total matching chunks (before limit)
    pub total_count: usize,
    
    /// Query processing time in milliseconds
    pub query_time_ms: u64,
    
    /// Retrieval mode used
    pub mode: RetrievalMode,
}

/// Common trait for all retrievers
#[async_trait::async_trait]
pub trait Retriever: Send + Sync {
    /// Retrieve chunks matching the query
    async fn retrieve(&self, request: &SearchRequest) -> Result<Vec<RetrievedChunk>>;
    
    /// Get the retrieval mode
    fn mode(&self) -> RetrievalMode;
}
