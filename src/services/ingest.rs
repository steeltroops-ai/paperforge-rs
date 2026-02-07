use crate::db::Repository;
use crate::embeddings::Embedder;
use crate::errors::AppError;
use uuid::Uuid;

use std::sync::Arc;

pub struct IngestService {
    repo: Repository,
    embedder: Arc<dyn Embedder>,
}

impl IngestService {
    pub fn new(repo: Repository, embedder: Arc<dyn Embedder>) -> Self {
        Self { repo, embedder }
    }

    pub async fn ingest_paper(&self, title: String, abstract_text: String, source: Option<String>) -> Result<Uuid, AppError> {
        // 1. Create Paper record
        let paper_id = self.repo.create_paper(title.clone(), abstract_text.clone(), source).await?;

        // 2. Chunk the abstract (simple implementation for MVP)
        // In production, use `text-splitter` crate for token-aware splitting.
        let chunks: Vec<String> = abstract_text
            .split_inclusive(|c| c == '.' || c == '\n') // Split by sentences/newlines roughly
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
            
        // If chunks are too small, they might be merged, but for MVP we take sentences.

        // 3. Generate Embeddings for chunks
        let embeddings = self.embedder.embed_documents(chunks.clone()).await?;

        // 4. Prepare data for bulk insert
        let chunk_data: Vec<(i32, String, Vec<f32>, i32)> = chunks.into_iter().zip(embeddings.into_iter()).enumerate()
            .map(|(idx, (content, embedding))| {
                let token_count = content.split_whitespace().count() as i32;
                (idx as i32, content, embedding, token_count)
            })
            .collect();

        // 5. Store chunks
        self.repo.create_chunks(paper_id, chunk_data).await?;

        // 6. Record Metrics
        metrics::counter!("paperforge_ingest_papers_total").increment(1);
        metrics::counter!("paperforge_ingest_chunks_total").increment(chunks.len() as u64);

        Ok(paper_id)
    }
}
