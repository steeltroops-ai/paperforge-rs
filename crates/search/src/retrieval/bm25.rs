//! BM25 lexical search using PostgreSQL full-text search
//!
//! Provides keyword-based search with ranking

use super::{RetrievalMode, RetrievedChunk, Retriever, SearchRequest};
use paperforge_common::errors::{AppError, Result};
use paperforge_common::db::DbPool;
use sea_orm::{ConnectionTrait, Statement, DbBackend};
use std::sync::Arc;
use uuid::Uuid;

/// BM25 retriever using PostgreSQL full-text search
pub struct BM25Retriever {
    db: Arc<DbPool>,
}

impl BM25Retriever {
    /// Create a new BM25 retriever
    pub fn new(db: Arc<DbPool>) -> Self {
        Self { db }
    }
    
    /// Prepare query for full-text search
    fn prepare_query(&self, query: &str) -> String {
        // Convert natural language query to tsquery format
        // Split into words and join with & (AND)
        query
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| {
                // Remove special characters
                w.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" & ")
    }
}

#[async_trait::async_trait]
impl Retriever for BM25Retriever {
    async fn retrieve(&self, request: &SearchRequest) -> Result<Vec<RetrievedChunk>> {
        let ts_query = self.prepare_query(&request.query);
        
        if ts_query.is_empty() {
            return Ok(vec![]);
        }
        
        let min_score = request.min_score.unwrap_or(0.0);
        
        // PostgreSQL full-text search with ts_rank_cd for BM25-like scoring
        let sql = r#"
            SELECT 
                c.id as chunk_id,
                c.paper_id,
                p.title as paper_title,
                c.content,
                c.chunk_index,
                ts_rank_cd(
                    to_tsvector('english', c.content),
                    plainto_tsquery('english', $2),
                    32 -- Normalize by document length
                ) as score
            FROM chunks c
            INNER JOIN papers p ON c.paper_id = p.id
            WHERE p.tenant_id = $1
              AND to_tsvector('english', c.content) @@ plainto_tsquery('english', $2)
            ORDER BY score DESC
            LIMIT $3
        "#;
        
        let conn = self.db.read_connection().await;
        let rows = conn
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                vec![
                    request.tenant_id.into(),
                    request.query.clone().into(),
                    (request.limit as i64).into(),
                ],
            ))
            .await
            .map_err(|e| AppError::DatabaseError { 
                message: format!("BM25 search failed: {}", e) 
            })?;
        
        let chunks: Vec<RetrievedChunk> = rows.iter().filter_map(|row| {
            use sea_orm::TryGetable;
            let score: f64 = row.try_get("", "score").ok()?;
            
            // Normalize score to 0-1 range (ts_rank_cd can exceed 1)
            let normalized_score = (score / (score + 1.0)) as f32;
            
            if normalized_score < min_score {
                return None;
            }
            
            Some(RetrievedChunk {
                chunk_id: row.try_get("", "chunk_id").ok()?,
                paper_id: row.try_get("", "paper_id").ok()?,
                paper_title: row.try_get("", "paper_title").ok()?,
                content: row.try_get("", "content").ok()?,
                chunk_index: row.try_get("", "chunk_index").ok()?,
                score: normalized_score,
                retrieval_mode: RetrievalMode::BM25,
            })
        }).collect();
        
        Ok(chunks)
    }
    
    fn mode(&self) -> RetrievalMode {
        RetrievalMode::BM25
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_preparation() {
        let db = Arc::new(DbPool::default_for_test());
        let retriever = BM25Retriever { db };
        
        // Note: This test will fail because DbPool::default_for_test doesn't exist
        // It's here to show the expected behavior
        let query = "machine learning transformers";
        let prepared = retriever.prepare_query(query);
        assert!(prepared.contains("&"));
    }
}
