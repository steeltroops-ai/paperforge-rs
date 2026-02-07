//! PageRank-based citation scoring
//!
//! Implements a simplified PageRank algorithm for ranking papers

use super::{CitationGraph, ScoredPaper};
use std::collections::HashMap;
use uuid::Uuid;

/// PageRank configuration
#[derive(Debug, Clone)]
pub struct PageRankConfig {
    /// Damping factor (typically 0.85)
    pub damping: f32,
    
    /// Maximum iterations
    pub max_iterations: usize,
    
    /// Convergence threshold
    pub epsilon: f32,
}

impl Default for PageRankConfig {
    fn default() -> Self {
        Self {
            damping: 0.85,
            max_iterations: 100,
            epsilon: 1e-6,
        }
    }
}

/// PageRank scorer for papers
pub struct PageRankScorer {
    config: PageRankConfig,
}

impl PageRankScorer {
    /// Create a new scorer
    pub fn new(config: PageRankConfig) -> Self {
        Self { config }
    }
    
    /// Compute PageRank scores for all papers
    pub fn compute(&self, graph: &CitationGraph) -> HashMap<Uuid, f32> {
        let n = graph.node_count();
        if n == 0 {
            return HashMap::new();
        }
        
        let n_f32 = n as f32;
        let initial_score = 1.0 / n_f32;
        let damping = self.config.damping;
        let teleport = (1.0 - damping) / n_f32;
        
        // Initialize scores
        let nodes: Vec<Uuid> = graph.nodes().cloned().collect();
        let mut scores: HashMap<Uuid, f32> = nodes.iter()
            .map(|&id| (id, initial_score))
            .collect();
        
        // Precompute outgoing counts
        let out_counts: HashMap<Uuid, usize> = nodes.iter()
            .map(|&id| (id, graph.reference_count(id)))
            .collect();
        
        // Iterative computation
        for _ in 0..self.config.max_iterations {
            let mut new_scores: HashMap<Uuid, f32> = HashMap::with_capacity(n);
            let mut max_diff: f32 = 0.0;
            
            for &node in &nodes {
                // Sum contributions from papers citing this one
                let citations = graph.get_citations(node);
                let citation_sum: f32 = citations.iter()
                    .map(|&citing| {
                        let citing_score = scores.get(&citing).copied().unwrap_or(0.0);
                        let citing_out = *out_counts.get(&citing).unwrap_or(&1) as f32;
                        citing_score / citing_out
                    })
                    .sum();
                
                let new_score = teleport + damping * citation_sum;
                
                let old_score = scores.get(&node).copied().unwrap_or(0.0);
                max_diff = max_diff.max((new_score - old_score).abs());
                
                new_scores.insert(node, new_score);
            }
            
            scores = new_scores;
            
            // Check convergence
            if max_diff < self.config.epsilon {
                break;
            }
        }
        
        // Normalize to 0-1 range
        let max_score = scores.values().cloned().fold(0.0f32, f32::max);
        if max_score > 0.0 {
            for score in scores.values_mut() {
                *score /= max_score;
            }
        }
        
        scores
    }
    
    /// Score and rank papers
    pub fn rank(&self, graph: &CitationGraph, limit: usize) -> Vec<ScoredPaper> {
        let scores = self.compute(graph);
        
        let mut papers: Vec<ScoredPaper> = scores.iter()
            .map(|(&paper_id, &authority_score)| {
                ScoredPaper {
                    paper_id,
                    title: graph.get_title(paper_id).cloned().unwrap_or_default(),
                    authority_score,
                    citation_count: graph.citation_count(paper_id),
                    reference_count: graph.reference_count(paper_id),
                }
            })
            .collect();
        
        // Sort by authority score descending
        papers.sort_by(|a, b| {
            b.authority_score.partial_cmp(&a.authority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        papers.truncate(limit);
        papers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pagerank_basic() {
        let mut graph = CitationGraph::new();
        
        // Create a simple graph:
        // A -> B -> C
        //      ^
        //      D
        // B should have highest score (most citations)
        
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let c = Uuid::from_u128(3);
        let d = Uuid::from_u128(4);
        
        graph.add_edge(a, b);
        graph.add_edge(b, c);
        graph.add_edge(d, b);
        
        let scorer = PageRankScorer::new(PageRankConfig::default());
        let scores = scorer.compute(&graph);
        
        // B should have a high score (cited by A and D)
        let b_score = scores.get(&b).copied().unwrap_or(0.0);
        let a_score = scores.get(&a).copied().unwrap_or(0.0);
        
        assert!(b_score > a_score, "B should rank higher than A");
    }
    
    #[test]
    fn test_pagerank_empty_graph() {
        let graph = CitationGraph::new();
        let scorer = PageRankScorer::new(PageRankConfig::default());
        let scores = scorer.compute(&graph);
        
        assert!(scores.is_empty());
    }
}
