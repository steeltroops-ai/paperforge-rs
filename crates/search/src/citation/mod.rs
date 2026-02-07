//! Citation propagation scoring
//!
//! Implements PageRank-inspired scoring for papers based on citation graph

mod graph;
mod pagerank;

pub use graph::{CitationGraph, CitationEdge};
pub use pagerank::{PageRankScorer, PageRankConfig};

use paperforge_common::errors::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Paper with citation score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredPaper {
    /// Paper ID
    pub paper_id: Uuid,
    
    /// Paper title
    pub title: String,
    
    /// Citation-based authority score (0.0 - 1.0)
    pub authority_score: f32,
    
    /// Number of incoming citations
    pub citation_count: usize,
    
    /// Number of outgoing references
    pub reference_count: usize,
}

/// Citation traversal result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    /// Root paper ID
    pub root_paper_id: Uuid,
    
    /// Papers cited by root (forward traversal)
    pub references: Vec<ScoredPaper>,
    
    /// Papers citing root (backward traversal)
    pub citations: Vec<ScoredPaper>,
    
    /// Traversal depth
    pub depth: usize,
}
