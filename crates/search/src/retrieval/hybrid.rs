//! Hybrid retrieval combining vector and BM25 search
//!
//! Uses RRF fusion to combine results from both retrievers

use super::{
    bm25::BM25Retriever,
    fusion::RRFusion,
    vector::VectorRetriever,
    RetrievalMode, RetrievedChunk, Retriever, SearchRequest,
};
use paperforge_common::errors::Result;
use paperforge_common::db::DbPool;
use std::sync::Arc;

/// Hybrid retriever combining vector and BM25
pub struct HybridRetriever {
    vector: VectorRetriever,
    bm25: BM25Retriever,
    fusion: RRFusion,
}

impl HybridRetriever {
    /// Create a new hybrid retriever
    pub fn new(db: Arc<DbPool>) -> Self {
        Self {
            vector: VectorRetriever::new(db.clone()),
            bm25: BM25Retriever::new(db),
            fusion: RRFusion::default(),
        }
    }
    
    /// Create with custom fusion weights
    pub fn with_weights(db: Arc<DbPool>, vector_weight: f32, bm25_weight: f32) -> Self {
        Self {
            vector: VectorRetriever::new(db.clone()),
            bm25: BM25Retriever::new(db),
            fusion: RRFusion::with_weights(vector_weight, bm25_weight),
        }
    }
}

#[async_trait::async_trait]
impl Retriever for HybridRetriever {
    async fn retrieve(&self, request: &SearchRequest) -> Result<Vec<RetrievedChunk>> {
        // Fetch more results from each retriever for better fusion
        let expanded_limit = request.limit * 2;
        
        let mut vector_request = request.clone();
        vector_request.limit = expanded_limit;
        vector_request.min_score = None; // We'll filter after fusion
        
        let mut bm25_request = request.clone();
        bm25_request.limit = expanded_limit;
        bm25_request.min_score = None;
        
        // Execute both searches in parallel
        let (vector_results, bm25_results) = tokio::join!(
            self.vector.retrieve(&vector_request),
            self.bm25.retrieve(&bm25_request)
        );
        
        let vector_results = vector_results.unwrap_or_default();
        let bm25_results = bm25_results.unwrap_or_default();
        
        // Fuse results using RRF
        let fused = self.fusion.fuse(vector_results, bm25_results, request.limit);
        
        // Apply min_score filter if specified
        let min_score = request.min_score.unwrap_or(0.0);
        let chunks: Vec<RetrievedChunk> = fused
            .into_iter()
            .filter(|r| r.chunk.score >= min_score)
            .map(|r| r.chunk)
            .collect();
        
        Ok(chunks)
    }
    
    fn mode(&self) -> RetrievalMode {
        RetrievalMode::Hybrid
    }
}
