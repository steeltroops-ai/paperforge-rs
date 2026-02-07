//! Embedding worker processor
//!
//! Processes embedding jobs: generates vectors and stores them in the database.

use paperforge_common::db::{DbPool, Repository, models::JobStatus};
use paperforge_common::embeddings::Embedder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Message received from embedding queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingJob {
    pub job_id: Uuid,
    pub paper_id: Uuid,
    pub chunks: Vec<ChunkData>,
    pub embedding_model: String,
}

/// Chunk data for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub index: i32,
    pub content: String,
    pub token_count: i32,
}

/// Embedding processor configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Batch size for embedding API calls
    pub batch_size: usize,
    /// Embedding model version for tracking
    pub embedding_version: i32,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            embedding_version: 1,
        }
    }
}

/// Embedding worker processor
pub struct EmbeddingProcessor {
    repository: Repository,
    embedder: Arc<dyn Embedder>,
    config: EmbeddingConfig,
}

impl EmbeddingProcessor {
    pub fn new(
        db_pool: DbPool,
        embedder: Arc<dyn Embedder>,
        config: EmbeddingConfig,
    ) -> Self {
        Self {
            repository: Repository::new(db_pool),
            embedder,
            config,
        }
    }

    /// Process an embedding job
    #[instrument(skip(self, job), fields(job_id = %job.job_id, paper_id = %job.paper_id))]
    pub async fn process_job(&self, job: EmbeddingJob) -> Result<(), EmbeddingError> {
        info!(
            chunk_count = job.chunks.len(),
            model = %job.embedding_model,
            "Processing embedding job"
        );

        let total_chunks = job.chunks.len();
        let mut processed = 0;
        let mut all_chunk_data = Vec::with_capacity(total_chunks);

        // Process chunks in batches
        for batch in job.chunks.chunks(self.config.batch_size) {
            debug!(
                batch_size = batch.len(),
                processed = processed,
                total = total_chunks,
                "Processing batch"
            );

            // Extract texts for embedding
            let texts: Vec<String> = batch.iter().map(|c| c.content.clone()).collect();

            // Generate embeddings
            let embeddings = self
                .embedder
                .embed_batch(&texts)
                .await
                .map_err(|e| EmbeddingError::EmbeddingFailed(e.to_string()))?;

            // Pair chunks with embeddings
            for (chunk, embedding) in batch.iter().zip(embeddings.into_iter()) {
                all_chunk_data.push((
                    chunk.index,
                    chunk.content.clone(),
                    embedding,
                    chunk.token_count,
                ));
            }

            processed += batch.len();

            // Update job progress
            if let Err(e) = self
                .repository
                .update_job_progress(job.job_id, processed as i32)
                .await
            {
                warn!(error = %e, "Failed to update job progress");
            }
        }

        // Store all chunks in database
        info!("Storing {} chunks in database...", all_chunk_data.len());

        self.repository
            .create_chunks(
                job.paper_id,
                all_chunk_data,
                &job.embedding_model,
                self.config.embedding_version,
            )
            .await
            .map_err(|e| EmbeddingError::DatabaseError(e.to_string()))?;

        // Mark job as completed
        self.repository
            .update_job_status(job.job_id, JobStatus::Completed, None, None, None)
            .await
            .map_err(|e| EmbeddingError::DatabaseError(e.to_string()))?;

        info!("Embedding job completed successfully");

        Ok(())
    }

    /// Process a single chunk (for testing)
    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.embedder
            .embed(text)
            .await
            .map_err(|e| EmbeddingError::EmbeddingFailed(e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<paperforge_common::errors::AppError> for EmbeddingError {
    fn from(e: paperforge_common::errors::AppError) -> Self {
        EmbeddingError::DatabaseError(e.to_string())
    }
}
