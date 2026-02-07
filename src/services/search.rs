use crate::db::Repository;
use crate::embeddings::Embedder;
use crate::errors::AppError;
use uuid::Uuid;
use serde::Serialize;
use crate::db::models::ChunkModel;

use std::sync::Arc;

pub struct SearchService {
    repo: Repository,
    embedder: Arc<dyn Embedder>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub chunk_id: Uuid,
    pub paper_id: Uuid,
    pub content: String,
    pub similarity_score: f64, // 1 - distance
    pub token_count: i32,
}

impl SearchService {
    pub fn new(repo: Repository, embedder: Arc<dyn Embedder>) -> Self {
        Self { repo, embedder }
    }

    pub async fn query(&self, query_text: String, limit: u64, hybrid: bool) -> Result<Vec<SearchResult>, AppError> {
        // 1. Generate query embedding
        let embedding = self.embedder.embed_query(&query_text).await?;

        // 2. Search DB (Hybrid or Pure Vector)
        let results = if hybrid {
            self.repo.search_hybrid(query_text, embedding, limit).await?
        } else {
            self.repo.search_similar_chunks(embedding, limit).await?
        };

        // 3. Transform to SearchResult
        // Note: Distance/Score handling. 
        // search_similar_chunks returns (model, distance) where distance = 1 - similarity (usually)
        // search_hybrid returns (model, score) where score is higher = better
        
        let mapped_results = results.into_iter().map(|(chunk, score_or_distance)| {
            let similarity_score = if hybrid {
                score_or_distance // It's already a score (0.7*vec + 0.3*text)
            } else {
                1.0 - score_or_distance // It's a distance
            };

            SearchResult {
                chunk_id: chunk.id,
                paper_id: chunk.paper_id,
                content: chunk.content,
                similarity_score,
                token_count: chunk.token_count,
            }
        }).collect();

        metrics::counter!("paperforge_search_ops_total").increment(1);

        Ok(mapped_results)
    }
}
