//! Database repository for PaperForge-rs
//!
//! Provides a high-level interface for database operations including:
//! - Paper CRUD with idempotency
//! - Chunk storage with embeddings
//! - Hybrid search (vector + BM25 via RRF)
//! - Health checks

use sea_orm::{
    DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, 
    ActiveModelTrait, Set, DbErr, Statement, ConnectionTrait, 
    DbBackend, FromQueryResult
};
use uuid::Uuid;
use super::models::{paper, chunk, Paper, Chunk};
use crate::config::DatabaseConfig;
use std::time::Duration;

/// Default embedding model for new chunks
const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-ada-002";
const DEFAULT_EMBEDDING_VERSION: i32 = 1;

#[derive(Clone)]
pub struct Repository {
    db: DatabaseConnection,
}

#[derive(Debug, FromQueryResult)]
pub struct SearchResult {
    pub id: Uuid,
    pub paper_id: Uuid,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: i32,
    pub embedding_model: String,
    pub embedding_version: i32,
    pub created_at: chrono::DateTime<chrono::FixedOffset>, 
    pub distance: f64,
}

/// Simple chunk result for returning from search
#[derive(Debug, Clone)]
pub struct ChunkResult {
    pub id: Uuid,
    pub paper_id: Uuid,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: i32,
    pub embedding_model: String,
    pub embedding_version: i32,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

impl Repository {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, DbErr> {
        let mut opt = sea_orm::ConnectOptions::new(&config.url);
        opt.max_connections(config.max_connections)
           .min_connections(config.min_connections)
           .connect_timeout(Duration::from_secs(config.connect_timeout))
           .acquire_timeout(Duration::from_secs(config.connect_timeout))
           .idle_timeout(Duration::from_secs(600))
           .sqlx_logging(cfg!(debug_assertions)); // Only log SQL in debug mode
        
        let db = sea_orm::Database::connect(opt).await?;
        
        tracing::info!(
            max_connections = config.max_connections,
            min_connections = config.min_connections,
            "Database connection pool initialized"
        );
        
        Ok(Self { db })
    }
    
    /// Ping the database to verify connectivity
    /// Used by health checks
    pub async fn ping(&self) -> Result<(), DbErr> {
        let stmt = Statement::from_string(DbBackend::Postgres, "SELECT 1".to_string());
        self.db.execute(stmt).await?;
        Ok(())
    }
    
    /// Find paper by idempotency key (for deduplication)
    /// Returns the existing paper if found
    pub async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Paper>, DbErr> {
        paper::Entity::find()
            .filter(paper::Column::IdempotencyKey.eq(key))
            .one(&self.db)
            .await
    }

    /// Create a new paper with idempotency key
    /// 
    /// Returns the paper_id. Call find_by_idempotency_key first to check for duplicates.
    pub async fn create_paper(
        &self, 
        title: String, 
        abstract_text: String, 
        source: Option<String>,
        idempotency_key: String,
    ) -> Result<Uuid, DbErr> {
        let id = Uuid::new_v4();
        let paper = paper::ActiveModel {
            id: Set(id),
            title: Set(title),
            abstract_text: Set(abstract_text),
            source: Set(source),
            idempotency_key: Set(Some(idempotency_key)),
            published_at: Set(Some(chrono::Utc::now().into())),
            created_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        };
        
        paper.insert(&self.db).await?;
        Ok(id)
    }
    
    /// Get a paper by ID
    pub async fn get_paper(&self, id: Uuid) -> Result<Option<Paper>, DbErr> {
        paper::Entity::find_by_id(id).one(&self.db).await
    }

    /// Create chunks with embeddings using parameterized queries
    /// 
    /// Tracks embedding model and version for future migration support.
    pub async fn create_chunks(
        &self, 
        paper_id: Uuid, 
        chunks: Vec<(i32, String, Vec<f32>, i32)>,
    ) -> Result<(), DbErr> {
        self.create_chunks_with_model(
            paper_id, 
            chunks, 
            DEFAULT_EMBEDDING_MODEL,
            DEFAULT_EMBEDDING_VERSION
        ).await
    }
    
    /// Create chunks with specific embedding model/version
    pub async fn create_chunks_with_model(
        &self, 
        paper_id: Uuid, 
        chunks: Vec<(i32, String, Vec<f32>, i32)>,
        embedding_model: &str,
        embedding_version: i32,
    ) -> Result<(), DbErr> {
        for (index, content, embedding, token_count) in chunks {
            let chunk_id = Uuid::new_v4();
            
            // Convert Vec<f32> to pgvector string format "[1.0, 2.0, ...]"
            let embedding_str = format!(
                "[{}]", 
                embedding.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            
            // Use parameterized query for safety
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                INSERT INTO chunks (id, paper_id, chunk_index, content, embedding, embedding_model, embedding_version, token_count, created_at)
                VALUES ($1, $2, $3, $4, $5::vector, $6, $7, $8, NOW())
                "#,
                vec![
                    chunk_id.into(),
                    paper_id.into(),
                    index.into(),
                    content.into(),
                    embedding_str.into(),
                    embedding_model.into(),
                    embedding_version.into(),
                    token_count.into(),
                ],
            );
            
            self.db.execute(stmt).await?;
        }
        
        Ok(())
    }

    /// Hybrid Search using Reciprocal Rank Fusion (RRF)
    /// 
    /// Combines vector similarity and BM25 text search using RRF formula:
    /// score = sum(1 / (k + rank_i)) for each ranking
    /// 
    /// This is more robust than simple weighted sums.
    pub async fn search_hybrid(
        &self, 
        query_text: String, 
        query_embedding: Vec<f32>, 
        limit: u64
    ) -> Result<Vec<(ChunkResult, f64)>, DbErr> {
        let embedding_str = format!(
            "[{}]", 
            query_embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        // RRF-based hybrid search
        // k = 60 is the standard constant (higher = more uniform blending)
        let sql = format!(
            r#"
            WITH vector_search AS (
                SELECT id, paper_id, chunk_index, content, token_count, embedding_model, embedding_version, created_at,
                       ROW_NUMBER() OVER (ORDER BY embedding <=> '{embedding}'::vector) as vector_rank
                FROM chunks
                ORDER BY embedding <=> '{embedding}'::vector
                LIMIT {limit_extended}
            ),
            text_search AS (
                SELECT id, paper_id, chunk_index, content, token_count, embedding_model, embedding_version, created_at,
                       ROW_NUMBER() OVER (ORDER BY ts_rank_cd(to_tsvector('english', content), plainto_tsquery('english', $1)) DESC) as text_rank
                FROM chunks
                WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $1)
                LIMIT {limit_extended}
            ),
            rrf_scores AS (
                SELECT 
                    COALESCE(v.id, t.id) as id,
                    COALESCE(v.paper_id, t.paper_id) as paper_id,
                    COALESCE(v.chunk_index, t.chunk_index) as chunk_index,
                    COALESCE(v.content, t.content) as content,
                    COALESCE(v.token_count, t.token_count) as token_count,
                    COALESCE(v.embedding_model, t.embedding_model) as embedding_model,
                    COALESCE(v.embedding_version, t.embedding_version) as embedding_version,
                    COALESCE(v.created_at, t.created_at) as created_at,
                    (COALESCE(1.0 / (60 + v.vector_rank), 0.0) + 
                     COALESCE(1.0 / (60 + t.text_rank), 0.0)) as rrf_score
                FROM vector_search v
                FULL OUTER JOIN text_search t ON v.id = t.id
            )
            SELECT id, paper_id, chunk_index, content, token_count, embedding_model, embedding_version, created_at, rrf_score as hybrid_score
            FROM rrf_scores
            ORDER BY rrf_score DESC
            LIMIT {limit}
            "#,
            embedding = embedding_str,
            limit_extended = limit * 2, // Get more candidates for fusion
            limit = limit
        );
        
        #[derive(Debug, FromQueryResult)]
        struct HybridResult {
            id: Uuid,
            paper_id: Uuid,
            chunk_index: i32,
            content: String,
            token_count: i32,
            embedding_model: String,
            embedding_version: i32,
            created_at: chrono::DateTime<chrono::FixedOffset>,
            hybrid_score: f64,
        }

        let results: Vec<HybridResult> = HybridResult::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, &sql, vec![query_text.into()])
        )
        .all(&self.db)
        .await?;
        
        // Normalize RRF scores to 0-1 range
        let max_score = results.first().map(|r| r.hybrid_score).unwrap_or(1.0);
        
        Ok(results.into_iter().map(|r| {
            let normalized_score = if max_score > 0.0 { 
                r.hybrid_score / max_score 
            } else { 
                0.0 
            };
            
            (ChunkResult {
                id: r.id,
                paper_id: r.paper_id,
                chunk_index: r.chunk_index,
                content: r.content,
                token_count: r.token_count,
                embedding_model: r.embedding_model,
                embedding_version: r.embedding_version,
                created_at: r.created_at,
            }, normalized_score)
        }).collect())
    }
    
    /// Pure vector similarity search
    pub async fn search_similar_chunks(
        &self, 
        query_embedding: Vec<f32>, 
        limit: u64
    ) -> Result<Vec<(ChunkResult, f64)>, DbErr> {
        let embedding_str = format!(
            "[{}]", 
            query_embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let sql = format!(
            r#"
            SELECT id, paper_id, chunk_index, content, token_count, embedding_model, embedding_version, created_at, 
                   embedding <=> '{}'::vector as distance
            FROM chunks
            ORDER BY distance ASC
            LIMIT {}
            "#,
            embedding_str, limit
        );

        #[derive(Debug, FromQueryResult)]
        struct VectorResult {
            id: Uuid,
            paper_id: Uuid,
            chunk_index: i32,
            content: String,
            token_count: i32,
            embedding_model: String,
            embedding_version: i32,
            created_at: chrono::DateTime<chrono::FixedOffset>,
            distance: f64,
        }

        let results: Vec<VectorResult> = VectorResult::find_by_statement(
            Statement::from_string(DbBackend::Postgres, sql)
        )
        .all(&self.db)
        .await?;
        
        Ok(results.into_iter().map(|r| (
            ChunkResult {
                id: r.id,
                paper_id: r.paper_id,
                chunk_index: r.chunk_index,
                content: r.content,
                token_count: r.token_count,
                embedding_model: r.embedding_model,
                embedding_version: r.embedding_version,
                created_at: r.created_at,
            },
            r.distance
        )).collect())
    }
    
    /// Get connection pool stats for monitoring
    /// Note: sea_orm doesn't expose pool stats directly, this is a placeholder
    pub fn pool_stats(&self) -> PoolStats {
        // TODO: Access underlying sqlx pool for actual stats
        PoolStats {
            active: 0,
            idle: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub active: u32,
    pub idle: u32,
}
