//! Context Stitcher - Assembles coherent context windows
//!
//! Provides:
//! - Chunk grouping by paper
//! - Context window assembly
//! - Cross-reference detection
//! - Token budget management

use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Context window for a single paper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Paper ID
    pub paper_id: Uuid,
    
    /// Paper title
    pub paper_title: String,
    
    /// Stitched content (combined chunks)
    pub content: String,
    
    /// Chunk range (start_idx, end_idx)
    pub chunk_range: (i32, i32),
    
    /// Relevance score (average of constituent chunks)
    pub relevance_score: f32,
    
    /// Token count in this window
    pub token_count: usize,
}

/// Cross-reference between context windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    /// Source window index
    pub from_window: usize,
    
    /// Target window index
    pub to_window: usize,
    
    /// Reference type
    pub reference_type: ReferenceType,
    
    /// Strength of connection (0.0 - 1.0)
    pub strength: f32,
}

/// Types of cross-references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReferenceType {
    /// Direct citation
    Citation,
    /// Shared concept
    Concept,
    /// Similar methodology
    Method,
    /// Contradicting findings
    Contradiction,
}

/// Context stitcher configuration
#[derive(Debug, Clone)]
pub struct ContextStitcherConfig {
    /// Maximum token budget
    pub max_tokens: usize,
    
    /// Maximum windows to create
    pub max_windows: usize,
    
    /// Overlap size for stitching (characters)
    pub stitch_overlap: usize,
    
    /// Minimum score to include chunk
    pub min_chunk_score: f32,
}

impl Default for ContextStitcherConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4000,
            max_windows: 5,
            stitch_overlap: 100,
            min_chunk_score: 0.3,
        }
    }
}

/// Chunk data for stitching
#[derive(Debug, Clone)]
pub struct ChunkInput {
    pub chunk_id: Uuid,
    pub paper_id: Uuid,
    pub paper_title: String,
    pub content: String,
    pub chunk_index: i32,
    pub score: f32,
}

/// Context stitcher for assembling context windows
pub struct ContextStitcher {
    config: ContextStitcherConfig,
}

impl ContextStitcher {
    /// Create a new context stitcher
    pub fn new(config: ContextStitcherConfig) -> Self {
        Self { config }
    }
    
    /// Stitch chunks into context windows
    pub fn stitch(&self, chunks: Vec<ChunkInput>) -> Result<(Vec<ContextWindow>, Vec<CrossReference>)> {
        // Filter by minimum score
        let mut chunks: Vec<ChunkInput> = chunks
            .into_iter()
            .filter(|c| c.score >= self.config.min_chunk_score)
            .collect();
        
        // Sort by paper_id then chunk_index
        chunks.sort_by(|a, b| {
            a.paper_id.cmp(&b.paper_id)
                .then_with(|| a.chunk_index.cmp(&b.chunk_index))
        });
        
        // Group by paper
        let mut paper_groups: HashMap<Uuid, Vec<ChunkInput>> = HashMap::new();
        for chunk in chunks {
            paper_groups.entry(chunk.paper_id).or_default().push(chunk);
        }
        
        // Create windows for each paper
        let mut windows = Vec::new();
        let mut total_tokens = 0;
        
        for (paper_id, paper_chunks) in paper_groups {
            if windows.len() >= self.config.max_windows {
                break;
            }
            
            let window = self.create_window(paper_id, paper_chunks);
            
            // Check token budget
            if total_tokens + window.token_count > self.config.max_tokens {
                // Try to fit a smaller version
                let remaining = self.config.max_tokens.saturating_sub(total_tokens);
                if remaining > 500 {
                    let trimmed = self.trim_window(window, remaining);
                    total_tokens += trimmed.token_count;
                    windows.push(trimmed);
                }
                break;
            }
            
            total_tokens += window.token_count;
            windows.push(window);
        }
        
        // Detect cross-references
        let cross_refs = self.detect_cross_references(&windows);
        
        // Sort windows by relevance
        windows.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
        });
        
        Ok((windows, cross_refs))
    }
    
    /// Create a context window from paper chunks
    fn create_window(&self, paper_id: Uuid, mut chunks: Vec<ChunkInput>) -> ContextWindow {
        // Sort by chunk index
        chunks.sort_by_key(|c| c.chunk_index);
        
        let paper_title = chunks.first()
            .map(|c| c.paper_title.clone())
            .unwrap_or_default();
        
        // Calculate average score
        let relevance_score = if chunks.is_empty() {
            0.0
        } else {
            chunks.iter().map(|c| c.score).sum::<f32>() / chunks.len() as f32
        };
        
        // Get chunk range
        let chunk_start = chunks.first().map(|c| c.chunk_index).unwrap_or(0);
        let chunk_end = chunks.last().map(|c| c.chunk_index).unwrap_or(0);
        
        // Stitch content with overlap handling
        let content = self.stitch_chunks(&chunks);
        let token_count = self.estimate_tokens(&content);
        
        ContextWindow {
            paper_id,
            paper_title,
            content,
            chunk_range: (chunk_start, chunk_end),
            relevance_score,
            token_count,
        }
    }
    
    /// Stitch chunks together handling overlaps
    fn stitch_chunks(&self, chunks: &[ChunkInput]) -> String {
        if chunks.is_empty() {
            return String::new();
        }
        
        if chunks.len() == 1 {
            return chunks[0].content.clone();
        }
        
        let mut result = String::new();
        
        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 {
                result.push_str(&chunk.content);
            } else {
                // Check for overlap with previous chunk
                let prev_end = if result.len() > self.config.stitch_overlap {
                    &result[result.len() - self.config.stitch_overlap..]
                } else {
                    &result
                };
                
                let chunk_start = if chunk.content.len() > self.config.stitch_overlap {
                    &chunk.content[..self.config.stitch_overlap]
                } else {
                    &chunk.content
                };
                
                // Simple overlap detection
                if prev_end.contains(chunk_start) {
                    // Skip overlapping content
                    let overlap_len = chunk_start.len();
                    if chunk.content.len() > overlap_len {
                        result.push_str(&chunk.content[overlap_len..]);
                    }
                } else {
                    // Add separator and content
                    result.push_str("\n\n");
                    result.push_str(&chunk.content);
                }
            }
        }
        
        result
    }
    
    /// Trim window to fit token budget
    fn trim_window(&self, window: ContextWindow, max_tokens: usize) -> ContextWindow {
        let estimated_chars = max_tokens * 4; // ~4 chars per token
        
        let content = if window.content.len() > estimated_chars {
            window.content.chars().take(estimated_chars).collect()
        } else {
            window.content
        };
        
        let token_count = self.estimate_tokens(&content);
        
        ContextWindow {
            content,
            token_count,
            ..window
        }
    }
    
    /// Detect cross-references between windows
    fn detect_cross_references(&self, windows: &[ContextWindow]) -> Vec<CrossReference> {
        let mut refs = Vec::new();
        
        for (i, win_a) in windows.iter().enumerate() {
            for (j, win_b) in windows.iter().enumerate() {
                if i >= j {
                    continue;
                }
                
                // Check for shared concepts (simple keyword overlap)
                let strength = self.calculate_overlap(&win_a.content, &win_b.content);
                
                if strength > 0.2 {
                    refs.push(CrossReference {
                        from_window: i,
                        to_window: j,
                        reference_type: ReferenceType::Concept,
                        strength,
                    });
                }
            }
        }
        
        refs
    }
    
    /// Calculate keyword overlap between two texts
    fn calculate_overlap(&self, text_a: &str, text_b: &str) -> f32 {
        let words_a: std::collections::HashSet<_> = text_a
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 4)
            .collect();
        
        let words_b: std::collections::HashSet<_> = text_b
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 4)
            .collect();
        
        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }
        
        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();
        
        intersection as f32 / union as f32
    }
    
    /// Estimate token count (simple approximation)
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: 1 token ~= 4 characters
        text.len() / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_stitching() {
        let stitcher = ContextStitcher::new(ContextStitcherConfig::default());
        
        let chunks = vec![
            ChunkInput {
                chunk_id: Uuid::new_v4(),
                paper_id: Uuid::new_v4(),
                paper_title: "Test Paper".to_string(),
                content: "First chunk content.".to_string(),
                chunk_index: 0,
                score: 0.8,
            },
            ChunkInput {
                chunk_id: Uuid::new_v4(),
                paper_id: Uuid::new_v4(), // Different paper
                paper_title: "Another Paper".to_string(),
                content: "Second chunk content.".to_string(),
                chunk_index: 0,
                score: 0.7,
            },
        ];
        
        let (windows, _refs) = stitcher.stitch(chunks).unwrap();
        
        assert_eq!(windows.len(), 2);
    }
    
    #[test]
    fn test_token_estimation() {
        let stitcher = ContextStitcher::new(ContextStitcherConfig::default());
        
        let tokens = stitcher.estimate_tokens("This is a test string.");
        assert!(tokens > 0);
    }
}
