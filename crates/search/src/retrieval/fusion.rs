//! Reciprocal Rank Fusion (RRF) for combining search results
//!
//! RRF is a simple but effective fusion method that:
//! - Doesn't require score normalization
//! - Works well with different scoring distributions
//! - Is robust to outliers

use super::{RetrievedChunk, RetrievalMode};
use std::collections::HashMap;
use uuid::Uuid;

/// RRF fusion parameters
#[derive(Debug, Clone)]
pub struct RRFusion {
    /// Constant k (typically 60)
    pub k: f32,
    
    /// Weight for vector results
    pub vector_weight: f32,
    
    /// Weight for BM25 results
    pub bm25_weight: f32,
}

impl Default for RRFusion {
    fn default() -> Self {
        Self {
            k: 60.0,
            vector_weight: 0.6,
            bm25_weight: 0.4,
        }
    }
}

/// Result of fusion
#[derive(Debug, Clone)]
pub struct FusionResult {
    pub chunk: RetrievedChunk,
    pub vector_rank: Option<usize>,
    pub bm25_rank: Option<usize>,
    pub rrf_score: f32,
}

impl RRFusion {
    /// Create with custom weights
    pub fn with_weights(vector_weight: f32, bm25_weight: f32) -> Self {
        Self {
            k: 60.0,
            vector_weight,
            bm25_weight,
        }
    }
    
    /// Fuse vector and BM25 results using RRF
    pub fn fuse(
        &self,
        vector_results: Vec<RetrievedChunk>,
        bm25_results: Vec<RetrievedChunk>,
        limit: usize,
    ) -> Vec<FusionResult> {
        // Create a map of chunk_id -> (chunk, vector_rank, bm25_rank)
        let mut chunk_map: HashMap<Uuid, (RetrievedChunk, Option<usize>, Option<usize>)> = HashMap::new();
        
        // Add vector results with ranks
        for (rank, chunk) in vector_results.into_iter().enumerate() {
            chunk_map.insert(chunk.chunk_id, (chunk, Some(rank + 1), None));
        }
        
        // Add or update with BM25 results
        for (rank, chunk) in bm25_results.into_iter().enumerate() {
            match chunk_map.get_mut(&chunk.chunk_id) {
                Some((_, _, bm25_rank)) => {
                    *bm25_rank = Some(rank + 1);
                }
                None => {
                    chunk_map.insert(chunk.chunk_id, (chunk, None, Some(rank + 1)));
                }
            }
        }
        
        // Calculate RRF scores
        let mut results: Vec<FusionResult> = chunk_map
            .into_iter()
            .map(|(_, (mut chunk, vector_rank, bm25_rank))| {
                let vector_rrf = vector_rank
                    .map(|r| self.vector_weight / (self.k + r as f32))
                    .unwrap_or(0.0);
                
                let bm25_rrf = bm25_rank
                    .map(|r| self.bm25_weight / (self.k + r as f32))
                    .unwrap_or(0.0);
                
                let rrf_score = vector_rrf + bm25_rrf;
                
                // Update chunk score and mode
                chunk.score = rrf_score;
                chunk.retrieval_mode = RetrievalMode::Hybrid;
                
                FusionResult {
                    chunk,
                    vector_rank,
                    bm25_rank,
                    rrf_score,
                }
            })
            .collect();
        
        // Sort by RRF score descending
        results.sort_by(|a, b| {
            b.rrf_score.partial_cmp(&a.rrf_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Limit results
        results.truncate(limit);
        
        // Normalize scores to 0-1 range
        if let Some(max_score) = results.first().map(|r| r.rrf_score) {
            if max_score > 0.0 {
                for result in &mut results {
                    result.chunk.score = result.rrf_score / max_score;
                    result.rrf_score = result.rrf_score / max_score;
                }
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn make_chunk(id: u128, score: f32) -> RetrievedChunk {
        RetrievedChunk {
            chunk_id: Uuid::from_u128(id),
            paper_id: Uuid::from_u128(1),
            paper_title: "Test Paper".to_string(),
            content: "Test content".to_string(),
            chunk_index: 0,
            score,
            retrieval_mode: RetrievalMode::Vector,
        }
    }
    
    #[test]
    fn test_rrf_fusion() {
        let fusion = RRFusion::default();
        
        // Vector: [A (0.9), B (0.8), C (0.7)]
        // BM25:   [B (0.9), A (0.7), D (0.6)]
        // Expected: B should rank highest (appears in both)
        
        let vector = vec![
            make_chunk(1, 0.9), // A
            make_chunk(2, 0.8), // B
            make_chunk(3, 0.7), // C
        ];
        
        let mut bm25_b = make_chunk(2, 0.9);
        bm25_b.retrieval_mode = RetrievalMode::BM25;
        let mut bm25_a = make_chunk(1, 0.7);
        bm25_a.retrieval_mode = RetrievalMode::BM25;
        let mut bm25_d = make_chunk(4, 0.6);
        bm25_d.retrieval_mode = RetrievalMode::BM25;
        
        let bm25 = vec![bm25_b, bm25_a, bm25_d];
        
        let results = fusion.fuse(vector, bm25, 10);
        
        assert!(!results.is_empty());
        
        // B should be first (appears in both at good ranks)
        assert_eq!(results[0].chunk.chunk_id, Uuid::from_u128(2));
        
        // A should be second (appears in both)
        assert_eq!(results[1].chunk.chunk_id, Uuid::from_u128(1));
    }
}
