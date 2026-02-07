//! Text chunking module
//!
//! Splits text into semantic chunks for embedding.

use text_splitter::{ChunkConfig, TextSplitter};
use tracing::debug;

/// Configuration for text chunking
#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub chunk_overlap: usize,
    /// Minimum chunk size (smaller chunks are merged)
    pub min_chunk_size: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
            min_chunk_size: 100,
        }
    }
}

/// A text chunk with metadata
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// The chunk content
    pub content: String,
    /// Index of this chunk in the document
    pub index: i32,
    /// Approximate token count
    pub token_count: i32,
    /// Start character position in original text
    pub start_pos: usize,
    /// End character position in original text
    pub end_pos: usize,
}

/// Split text into chunks for embedding
pub fn chunk_text(text: &str, config: &ChunkingConfig) -> Vec<TextChunk> {
    let splitter = TextSplitter::new(ChunkConfig::new(config.chunk_size));
    
    let chunks: Vec<&str> = splitter.chunks(text).collect();
    
    debug!(
        input_len = text.len(),
        chunk_count = chunks.len(),
        chunk_size = config.chunk_size,
        "Text chunked"
    );

    let mut result = Vec::with_capacity(chunks.len());
    let mut pos = 0;

    for (index, chunk_text) in chunks.into_iter().enumerate() {
        // Find the actual position in the original text
        let start_pos = text[pos..].find(chunk_text).map(|p| pos + p).unwrap_or(pos);
        let end_pos = start_pos + chunk_text.len();
        
        // Skip chunks that are too small
        if chunk_text.len() < config.min_chunk_size {
            continue;
        }

        // Estimate token count (rough approximation: ~4 chars per token)
        let token_count = (chunk_text.len() / 4) as i32;

        result.push(TextChunk {
            content: chunk_text.to_string(),
            index: index as i32,
            token_count,
            start_pos,
            end_pos,
        });

        pos = end_pos;
    }

    // Re-index after filtering
    for (i, chunk) in result.iter_mut().enumerate() {
        chunk.index = i as i32;
    }

    result
}

/// Chunk text with overlap (sliding window)
pub fn chunk_text_with_overlap(text: &str, config: &ChunkingConfig) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let total_len = chars.len();
    
    if total_len == 0 {
        return chunks;
    }

    let mut start = 0;
    let mut index = 0;

    while start < total_len {
        let end = (start + config.chunk_size).min(total_len);
        let chunk_chars: String = chars[start..end].iter().collect();
        
        // Try to break at sentence boundary
        let chunk_text = if end < total_len {
            find_sentence_boundary(&chunk_chars)
        } else {
            chunk_chars
        };

        if chunk_text.len() >= config.min_chunk_size {
            let token_count = (chunk_text.len() / 4) as i32;
            
            chunks.push(TextChunk {
                content: chunk_text.clone(),
                index,
                token_count,
                start_pos: start,
                end_pos: start + chunk_text.len(),
            });
            
            index += 1;
        }

        // Move forward with overlap
        let advance = if config.chunk_overlap < config.chunk_size {
            config.chunk_size - config.chunk_overlap
        } else {
            config.chunk_size / 2
        };
        
        start += advance.max(1);
    }

    chunks
}

/// Find a good sentence boundary to break at
fn find_sentence_boundary(text: &str) -> String {
    // Look for sentence-ending punctuation near the end
    let sentence_endings = [". ", "! ", "? ", ".\n", "!\n", "?\n"];
    
    // Search in the last 20% of the text for a good break point
    let search_start = (text.len() as f64 * 0.8) as usize;
    let search_region = &text[search_start..];
    
    for ending in sentence_endings.iter() {
        if let Some(pos) = search_region.rfind(ending) {
            let break_pos = search_start + pos + ending.len();
            return text[..break_pos].to_string();
        }
    }
    
    // No good break found, return as-is
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chunking() {
        let text = "This is a test. ".repeat(100);
        let config = ChunkingConfig {
            chunk_size: 200,
            chunk_overlap: 50,
            min_chunk_size: 50,
        };
        
        let chunks = chunk_text(&text, &config);
        assert!(!chunks.is_empty());
        
        for chunk in &chunks {
            assert!(chunk.content.len() >= config.min_chunk_size);
        }
    }

    #[test]
    fn test_overlap_chunking() {
        let text = "Sentence one. Sentence two. Sentence three. Sentence four. Sentence five.";
        let config = ChunkingConfig {
            chunk_size: 30,
            chunk_overlap: 10,
            min_chunk_size: 10,
        };
        
        let chunks = chunk_text_with_overlap(&text, &config);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_empty_text() {
        let chunks = chunk_text("", &ChunkingConfig::default());
        assert!(chunks.is_empty());
    }
}
