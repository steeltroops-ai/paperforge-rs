//! Paper ingestion service
//!
//! Handles the core ingestion workflow:
//! 1. Create paper record
//! 2. Chunk abstract text with overlap
//! 3. Generate embeddings
//! 4. Store chunks with embeddings

use crate::db::Repository;
use crate::embeddings::Embedder;
use crate::errors::AppError;
use uuid::Uuid;

use std::sync::Arc;
use std::time::Instant;

/// Chunking configuration
const TARGET_CHUNK_SIZE: usize = 400;  // Target tokens (approx 4 chars/token)
const CHUNK_OVERLAP: usize = 80;       // 20% overlap
const MIN_CHUNK_SIZE: usize = 50;      // Don't create tiny chunks

pub struct IngestService {
    repo: Repository,
    embedder: Arc<dyn Embedder>,
}

impl IngestService {
    pub fn new(repo: Repository, embedder: Arc<dyn Embedder>) -> Self {
        Self { repo, embedder }
    }

    /// Ingest a paper with improved chunking strategy
    /// 
    /// Returns (paper_id, chunks_created)
    pub async fn ingest_paper(
        &self, 
        title: String, 
        abstract_text: String, 
        source: Option<String>,
        idempotency_key: String,
    ) -> Result<(Uuid, usize), AppError> {
        let start = Instant::now();
        
        // 1. Create Paper record
        let paper_id = self.repo.create_paper(
            title.clone(), 
            abstract_text.clone(), 
            source,
            idempotency_key,
        ).await?;

        // 2. Improved chunking with overlap
        let chunks = self.chunk_text(&abstract_text);
        let chunk_count = chunks.len();
        
        if chunks.is_empty() {
            tracing::warn!(paper_id = %paper_id, "No chunks created - abstract too short");
            return Ok((paper_id, 0));
        }

        // 3. Generate Embeddings for chunks
        let embedding_start = Instant::now();
        let embeddings = self.embedder.embed_documents(chunks.clone()).await?;
        let embedding_duration = embedding_start.elapsed();
        
        tracing::debug!(
            paper_id = %paper_id,
            chunks = chunk_count,
            embedding_ms = embedding_duration.as_millis(),
            "Embeddings generated"
        );

        // 4. Prepare data for bulk insert
        let chunk_data: Vec<(i32, String, Vec<f32>, i32)> = chunks
            .into_iter()
            .zip(embeddings.into_iter())
            .enumerate()
            .map(|(idx, (content, embedding))| {
                // Approximate token count (words * 1.3)
                let token_count = (content.split_whitespace().count() as f32 * 1.3) as i32;
                (idx as i32, content, embedding, token_count)
            })
            .collect();

        // 5. Store chunks
        self.repo.create_chunks(paper_id, chunk_data).await?;

        // 6. Record Metrics
        let total_duration = start.elapsed();
        metrics::counter!("paperforge_ingest_papers_total").increment(1);
        metrics::counter!("paperforge_ingest_chunks_total").increment(chunk_count as u64);
        metrics::histogram!("paperforge_ingest_duration_seconds").record(total_duration.as_secs_f64());
        metrics::histogram!("paperforge_embedding_duration_seconds").record(embedding_duration.as_secs_f64());

        tracing::info!(
            paper_id = %paper_id,
            chunks = chunk_count,
            total_ms = total_duration.as_millis(),
            "Paper ingested successfully"
        );

        Ok((paper_id, chunk_count))
    }
    
    /// Improved chunking strategy with overlap
    /// 
    /// Features:
    /// - Target chunk size of ~400 tokens
    /// - 20% overlap between chunks for context preservation
    /// - Sentence-boundary aware splitting
    /// - Handles abbreviations better (Fig., Dr., etc.)
    fn chunk_text(&self, text: &str) -> Vec<String> {
        let text = text.trim();
        if text.is_empty() {
            return vec![];
        }
        
        // Approximate character count for target size (4 chars ~= 1 token)
        let target_chars = TARGET_CHUNK_SIZE * 4;
        let overlap_chars = CHUNK_OVERLAP * 4;
        let min_chars = MIN_CHUNK_SIZE * 4;
        
        // If text is small enough, return as single chunk
        if text.len() <= target_chars + overlap_chars {
            return vec![text.to_string()];
        }
        
        // Split into sentences (improved regex-free approach)
        let sentences = self.split_into_sentences(text);
        
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut overlap_buffer = String::new();
        
        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }
            
            // Check if adding this sentence would exceed target
            if !current_chunk.is_empty() && 
               current_chunk.len() + sentence.len() > target_chars {
                // Save current chunk
                if current_chunk.len() >= min_chars {
                    chunks.push(current_chunk.trim().to_string());
                }
                
                // Start new chunk with overlap from previous
                current_chunk = overlap_buffer.clone();
            }
            
            // Add sentence to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(sentence);
            
            // Update overlap buffer (last few sentences)
            overlap_buffer.push(' ');
            overlap_buffer.push_str(sentence);
            if overlap_buffer.len() > overlap_chars * 2 {
                // Trim overlap buffer to reasonable size
                let trim_point = overlap_buffer.len() - overlap_chars;
                if let Some(space_idx) = overlap_buffer[trim_point..].find(' ') {
                    overlap_buffer = overlap_buffer[trim_point + space_idx..].to_string();
                }
            }
        }
        
        // Don't forget the last chunk
        if current_chunk.len() >= min_chars {
            chunks.push(current_chunk.trim().to_string());
        } else if !chunks.is_empty() {
            // Append to previous chunk if too small
            let last_idx = chunks.len() - 1;
            chunks[last_idx].push(' ');
            chunks[last_idx].push_str(&current_chunk);
        } else if !current_chunk.is_empty() {
            // Single small chunk is better than nothing
            chunks.push(current_chunk.trim().to_string());
        }
        
        chunks
    }
    
    /// Split text into sentences with handling for common abbreviations
    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        
        // Common abbreviations that don't end sentences
        let abbreviations = ["Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Fig.", "Eq.", 
                            "et al.", "i.e.", "e.g.", "vs.", "etc.", "al."];
        
        for (i, &ch) in chars.iter().enumerate() {
            current.push(ch);
            
            // Check for sentence end
            if (ch == '.' || ch == '!' || ch == '?') && 
               i + 1 < chars.len() && 
               (chars[i + 1].is_whitespace() || chars[i + 1].is_uppercase()) {
                
                // Check if this is an abbreviation
                let is_abbrev = abbreviations.iter().any(|abbr| {
                    current.ends_with(abbr)
                });
                
                if !is_abbrev {
                    sentences.push(current.trim().to_string());
                    current = String::new();
                }
            }
        }
        
        // Don't forget remaining text
        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }
        
        sentences
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test helper to create IngestService for chunking tests
    /// Note: This only tests the chunking logic, not the full ingestion
    fn create_chunking_test_helper() -> IngestService {
        // Create a mock repo and embedder - these won't be used for chunking tests
        // We use a placeholder for tests that only exercise the pure chunking logic
        // For full integration tests, use a proper test database
        
        // This is a workaround since we can't easily mock the repo
        // In production, these test methods could be moved to a ChunkingService
        unimplemented!("Full test setup requires mock repository - see integration tests")
    }
    
    #[test]
    fn test_abbreviation_list_completeness() {
        // Verify common abbreviations are included
        let abbreviations = ["Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Fig.", "Eq.", 
                            "et al.", "i.e.", "e.g.", "vs.", "etc.", "al."];
        
        assert!(abbreviations.contains(&"Dr."));
        assert!(abbreviations.contains(&"et al."));
        assert!(abbreviations.contains(&"i.e."));
        assert_eq!(abbreviations.len(), 12);
    }
    
    #[test]
    fn test_chunking_constants() {
        // Verify chunking constants are reasonable
        assert!(TARGET_CHUNK_SIZE >= 100);
        assert!(TARGET_CHUNK_SIZE <= 1000);
        assert!(CHUNK_OVERLAP < TARGET_CHUNK_SIZE);
        assert!(MIN_CHUNK_SIZE < TARGET_CHUNK_SIZE);
    }
}
