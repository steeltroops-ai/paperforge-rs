//! Vector similarity search using pgvector
//!
//! Provides semantic search via embedding similarity

use super::{RetrievalMode, RetrievedChunk, Retriever, SearchRequest};
use paperforge_common::errors::{AppError, Result};
use paperforge_common::db::DbPool;
use sea_orm::{ConnectionTrait, Statement, FromQueryResult, DbBackend};
use std::sync::Arc;
use uuid::Uuid;

/// Vector retriever using pgvector
pub struct VectorRetriever {
    db: Arc<DbPool>,
}

impl VectorRetriever {
    /// Create a new vector retriever
    pub fn new(db: Arc<DbPool>) -> Self {
        Self { db }
    }
    
    /// Build the vector search query
    fn build_query(
        &self,
        tenant_id: Uuid,
        embedding: &[f32],
        limit: usize,
        min_score: f32,
        paper_ids: Option<&[Uuid]>,
    ) -> (String, Vec<sea_orm::Value>) {
        let embedding_str = format!(
            "[{}]",
            embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        let mut sql = format!(
            r#"
            SELECT 
                c.id as chunk_id,
                c.paper_id,
                p.title as paper_title,
                c.content,
                c.chunk_index,
                1 - (c.embedding <=> '{embedding}'::vector) as score
            FROM chunks c
            INNER JOIN papers p ON c.paper_id = p.id
            WHERE p.tenant_id = $1
              AND 1 - (c.embedding <=> '{embedding}'::vector) >= $2
            "#,
            embedding = embedding_str
        );
        
        if paper_ids.is_some() {
            sql.push_str(" AND c.paper_id = ANY($3)");
        }
        
        sql.push_str(&format!(
            r#"
            ORDER BY c.embedding <=> '{}'::vector
            LIMIT {}
            "#,
            embedding_str, limit
        ));
        
        (sql, vec![])
    }
}

/// Query result row
#[derive(Debug, FromQueryResult)]
struct ChunkRow {
    chunk_id: Uuid,
    paper_id: Uuid,
    paper_title: String,
    content: String,
    chunk_index: i32,
    score: f64,
}

#[async_trait::async_trait]
impl Retriever for VectorRetriever {
    async fn retrieve(&self, request: &SearchRequest) -> Result<Vec<RetrievedChunk>> {
        let embedding = request.query_embedding.as_ref()
            .ok_or_else(|| AppError::ValidationFailed {
                message: "Vector search requires query embedding".to_string(),
            })?;
        
        let min_score = request.min_score.unwrap_or(0.0);
        
        // Build embedding string for SQL
        let embedding_str = format!(
            "[{}]",
            embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        // Build SQL query
        let sql = format!(
            r#"
            SELECT 
                c.id as chunk_id,
                c.paper_id,
                p.title as paper_title,
                c.content,
                c.chunk_index,
                1 - (c.embedding <=> '{embedding}'::vector) as score
            FROM chunks c
            INNER JOIN papers p ON c.paper_id = p.id
            WHERE p.tenant_id = $1
              AND 1 - (c.embedding <=> '{embedding}'::vector) >= $2
            ORDER BY c.embedding <=> '{embedding}'::vector
            LIMIT $3
            "#,
            embedding = embedding_str
        );
        
        let conn = self.db.read_connection().await;
        let rows = conn
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                vec![
                    request.tenant_id.into(),
                    min_score.into(),
                    (request.limit as i64).into(),
                ],
            ))
            .await
            .map_err(|e| AppError::DatabaseError { 
                message: format!("Vector search failed: {}", e) 
            })?;
        
        let chunks = rows.iter().map(|row| {
            use sea_orm::TryGetable;
            RetrievedChunk {
                chunk_id: row.try_get("", "chunk_id").unwrap_or_default(),
                paper_id: row.try_get("", "paper_id").unwrap_or_default(),
                paper_title: row.try_get("", "paper_title").unwrap_or_default(),
                content: row.try_get("", "content").unwrap_or_default(),
                chunk_index: row.try_get("", "chunk_index").unwrap_or_default(),
                score: row.try_get::<f64, _>("", "score").unwrap_or_default() as f32,
                retrieval_mode: RetrievalMode::Vector,
            }
        }).collect();
        
        Ok(chunks)
    }
    
    fn mode(&self) -> RetrievalMode {
        RetrievalMode::Vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_embedding_format() {
        let embedding = vec![0.1, 0.2, 0.3];
        let formatted = format!(
            "[{}]",
            embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(formatted, "[0.1,0.2,0.3]");
    }
}
