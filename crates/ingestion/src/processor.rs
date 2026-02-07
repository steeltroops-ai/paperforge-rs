//! Ingestion processor
//!
//! Core logic for processing papers: PDF extraction, chunking, and queue dispatch.

use crate::chunker::{chunk_text, ChunkingConfig, TextChunk};
use crate::errors::IngestionError;
use crate::pdf::extract_text_from_pdf;
use paperforge_common::db::{DbPool, Repository};
use paperforge_common::queue::Queue;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Message sent to the embedding queue
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

/// Ingestion job message (received from SQS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionJobMessage {
    pub job_id: Uuid,
    pub tenant_id: Uuid,
    pub paper_id: Uuid,
    pub source_type: SourceType,
    pub source_path: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    LocalFile,
    S3,
    Url,
}

/// Ingestion processor
pub struct IngestionProcessor {
    repository: Repository,
    embedding_queue: Option<Arc<Queue>>,
    chunking_config: ChunkingConfig,
    embedding_model: String,
}

impl IngestionProcessor {
    pub fn new(
        db_pool: DbPool,
        embedding_queue: Option<Arc<Queue>>,
        chunking_config: ChunkingConfig,
        embedding_model: String,
    ) -> Self {
        Self {
            repository: Repository::new(db_pool),
            embedding_queue,
            chunking_config,
            embedding_model,
        }
    }

    /// Process a local PDF file directly (for testing without SQS)
    #[instrument(skip(self), fields(path = %path.display()))]
    pub async fn process_local_pdf(
        &self,
        path: &Path,
        tenant_id: Uuid,
        title: Option<String>,
    ) -> Result<(Uuid, Uuid, Vec<TextChunk>), IngestionError> {
        info!("Processing local PDF");

        // Create job
        let job = self
            .repository
            .create_job(tenant_id, None)
            .await
            .map_err(|e| IngestionError::DatabaseError(e.to_string()))?;

        let job_id = job.id;

        // Extract text from PDF
        info!("Extracting text from PDF...");
        let text = extract_text_from_pdf(path)?;

        // Get title from metadata or filename
        let paper_title = title.unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        });

        // Create paper record
        let paper = self
            .repository
            .create_paper(
                tenant_id,
                paper_title,
                text.chars().take(500).collect(), // First 500 chars as abstract
                Some(path.display().to_string()),
                None,
                serde_json::json!({
                    "source": "local_file",
                    "file_path": path.display().to_string(),
                }),
                None,
            )
            .await
            .map_err(|e| IngestionError::DatabaseError(e.to_string()))?;

        let paper_id = paper.id;

        // Update job with paper ID
        self.repository
            .update_job_status(
                job_id,
                paperforge_common::db::models::JobStatus::Chunking,
                Some(paper_id),
                None,
                None,
            )
            .await
            .map_err(|e| IngestionError::DatabaseError(e.to_string()))?;

        // Chunk the text
        info!("Chunking text...");
        let chunks = chunk_text(&text, &self.chunking_config);

        info!(chunk_count = chunks.len(), "Text chunked successfully");

        // Update job with chunk count
        self.repository
            .update_job_status(
                job_id,
                paperforge_common::db::models::JobStatus::Embedding,
                None,
                Some(chunks.len() as i32),
                None,
            )
            .await
            .map_err(|e| IngestionError::DatabaseError(e.to_string()))?;

        // Send to embedding queue if available
        if let Some(ref queue) = self.embedding_queue {
            let embedding_job = EmbeddingJob {
                job_id,
                paper_id,
                chunks: chunks
                    .iter()
                    .map(|c| ChunkData {
                        index: c.index,
                        content: c.content.clone(),
                        token_count: c.token_count,
                    })
                    .collect(),
                embedding_model: self.embedding_model.clone(),
            };

            queue
                .send(&embedding_job)
                .await
                .map_err(|e| IngestionError::QueueError(e.to_string()))?;

            info!("Embedding job sent to queue");
        } else {
            warn!("No embedding queue configured, chunks not sent for embedding");
        }

        Ok((job_id, paper_id, chunks))
    }

    /// Process an ingestion job from SQS
    #[instrument(skip(self, message), fields(job_id = %message.job_id))]
    pub async fn process_job(&self, message: IngestionJobMessage) -> Result<(), IngestionError> {
        info!("Processing ingestion job");

        match message.source_type {
            SourceType::LocalFile => {
                let path = Path::new(&message.source_path);
                if !path.exists() {
                    return Err(IngestionError::FileNotFound(message.source_path));
                }
                self.process_local_pdf(path, message.tenant_id, None)
                    .await?;
            }
            SourceType::S3 => {
                // TODO: Download from S3 first
                warn!("S3 source not yet implemented");
                return Err(IngestionError::ConfigError(
                    "S3 source not yet implemented".to_string(),
                ));
            }
            SourceType::Url => {
                // TODO: Download from URL first
                warn!("URL source not yet implemented");
                return Err(IngestionError::ConfigError(
                    "URL source not yet implemented".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Batch process all PDFs in a directory (for testing)
    #[instrument(skip(self), fields(dir = %dir.display()))]
    pub async fn process_directory(
        &self,
        dir: &Path,
        tenant_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, usize)>, IngestionError> {
        info!("Processing directory of PDFs");

        let mut results = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                match self.process_local_pdf(&path, tenant_id, None).await {
                    Ok((job_id, paper_id, chunks)) => {
                        info!(
                            job_id = %job_id,
                            paper_id = %paper_id,
                            chunk_count = chunks.len(),
                            "PDF processed successfully"
                        );
                        results.push((job_id, paper_id, chunks.len()));
                    }
                    Err(e) => {
                        error!(
                            path = %path.display(),
                            error = %e,
                            "Failed to process PDF"
                        );
                    }
                }
            }
        }

        info!(
            total = results.len(),
            "Directory processing complete"
        );

        Ok(results)
    }
}
