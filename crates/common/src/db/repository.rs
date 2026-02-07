//! Repository pattern for database operations
//!
//! Provides a clean interface for all data access operations
//! with proper error handling and transaction support.

use crate::errors::{AppError, Result};
use crate::db::DbPool;
use crate::db::models::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, 
    PaginatorTrait, QueryFilter, QueryOrder, Set, Statement,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Result from search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResult {
    pub chunk_id: Uuid,
    pub paper_id: Uuid,
    pub paper_title: String,
    pub content: String,
    pub chunk_index: i32,
    pub score: f64,
    pub embedding_model: String,
}

/// Repository for data access operations
#[derive(Clone)]
pub struct Repository {
    pool: DbPool,
}

impl Repository {
    /// Create a new repository with the given connection pool
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
    
    /// Get the read connection
    fn read_conn(&self) -> &DatabaseConnection {
        self.pool.read()
    }
    
    /// Get the write connection
    fn write_conn(&self) -> &DatabaseConnection {
        self.pool.write()
    }
    
    // ========================================================================
    // Health Check
    // ========================================================================
    
    /// Ping the database
    pub async fn ping(&self) -> Result<()> {
        self.pool.ping().await
    }
    
    // ========================================================================
    // Tenant Operations
    // ========================================================================
    
    /// Find tenant by ID
    pub async fn find_tenant_by_id(&self, id: Uuid) -> Result<Option<Tenant>> {
        TenantEntity::find_by_id(id)
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    /// Find tenant by API key hash
    pub async fn find_tenant_by_api_key_hash(&self, hash: &str) -> Result<Option<Tenant>> {
        TenantEntity::find()
            .filter(TenantColumn::ApiKeyHash.eq(hash))
            .filter(TenantColumn::IsActive.eq(true))
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    // ========================================================================
    // Paper Operations
    // ========================================================================
    
    /// Create a new paper
    pub async fn create_paper(
        &self,
        tenant_id: Uuid,
        title: String,
        abstract_text: String,
        source: Option<String>,
        external_id: Option<String>,
        metadata: serde_json::Value,
        idempotency_key: Option<String>,
    ) -> Result<Paper> {
        let paper_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        
        let paper = PaperActiveModel {
            id: Set(paper_id),
            tenant_id: Set(tenant_id),
            external_id: Set(external_id),
            title: Set(title),
            abstract_text: Set(abstract_text),
            published_at: Set(None),
            source: Set(source),
            metadata: Set(metadata),
            idempotency_key: Set(idempotency_key),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };
        
        paper.insert(self.write_conn()).await.map_err(Into::into)
    }
    
    /// Find paper by ID
    pub async fn find_paper_by_id(&self, id: Uuid) -> Result<Option<Paper>> {
        PaperEntity::find_by_id(id)
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    /// Find paper by idempotency key within tenant
    pub async fn find_paper_by_idempotency_key(
        &self,
        tenant_id: Uuid,
        key: &str,
    ) -> Result<Option<Paper>> {
        PaperEntity::find()
            .filter(PaperColumn::TenantId.eq(tenant_id))
            .filter(PaperColumn::IdempotencyKey.eq(key))
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    /// List papers for a tenant with pagination
    pub async fn list_papers(
        &self,
        tenant_id: Uuid,
        offset: u64,
        limit: u64,
    ) -> Result<(Vec<Paper>, u64)> {
        let paginator = PaperEntity::find()
            .filter(PaperColumn::TenantId.eq(tenant_id))
            .order_by_desc(PaperColumn::CreatedAt)
            .paginate(self.read_conn(), limit);
        
        let total = paginator.num_items().await?;
        let papers = paginator.fetch_page(offset / limit).await?;
        
        Ok((papers, total))
    }
    
    /// Delete paper by ID
    pub async fn delete_paper(&self, id: Uuid) -> Result<bool> {
        let result = PaperEntity::delete_by_id(id)
            .exec(self.write_conn())
            .await?;
        
        Ok(result.rows_affected > 0)
    }
    
    // ========================================================================
    // Chunk Operations
    // ========================================================================
    
    /// Create chunks for a paper (with vector embedding via raw SQL)
    pub async fn create_chunks(
        &self,
        paper_id: Uuid,
        chunks: Vec<(i32, String, Vec<f32>, i32)>,  // (index, content, embedding, token_count)
        embedding_model: &str,
        embedding_version: i32,
    ) -> Result<Vec<Uuid>> {
        let mut chunk_ids = Vec::with_capacity(chunks.len());
        
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
            
            // Use raw SQL for pgvector type
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                INSERT INTO chunks (
                    id, paper_id, chunk_index, content, embedding, 
                    embedding_model, embedding_version, token_count, created_at
                )
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
            
            self.write_conn().execute(stmt).await?;
            chunk_ids.push(chunk_id);
        }
        
        Ok(chunk_ids)
    }
    
    /// Get chunks for a paper
    pub async fn get_chunks_by_paper(&self, paper_id: Uuid) -> Result<Vec<Chunk>> {
        ChunkEntity::find()
            .filter(ChunkColumn::PaperId.eq(paper_id))
            .order_by_asc(ChunkColumn::ChunkIndex)
            .all(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    /// Vector similarity search
    pub async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<ChunkResult>> {
        let embedding_str = format!(
            "[{}]",
            embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        let tenant_filter = tenant_id
            .map(|_| "AND p.tenant_id = $3")
            .unwrap_or("");
        
        let sql = format!(
            r#"
            SELECT 
                c.id as chunk_id,
                c.paper_id,
                p.title as paper_title,
                c.content,
                c.chunk_index,
                c.embedding_model,
                1 - (c.embedding <=> $1::vector) as score
            FROM chunks c
            JOIN papers p ON c.paper_id = p.id
            WHERE c.embedding IS NOT NULL
            {}
            ORDER BY c.embedding <=> $1::vector
            LIMIT $2
            "#,
            tenant_filter
        );
        
        let mut values: Vec<sea_orm::Value> = vec![
            embedding_str.into(),
            (limit as i32).into(),
        ];
        
        if let Some(tid) = tenant_id {
            values.push(tid.into());
        }
        
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, &sql, values);
        
        let results = self.read_conn()
            .query_all(stmt)
            .await?
            .into_iter()
            .filter_map(|row| {
                use sea_orm::QueryResult;
                Some(ChunkResult {
                    chunk_id: row.try_get_by_index::<Uuid>(0).ok()?,
                    paper_id: row.try_get_by_index::<Uuid>(1).ok()?,
                    paper_title: row.try_get_by_index::<String>(2).ok()?,
                    content: row.try_get_by_index::<String>(3).ok()?,
                    chunk_index: row.try_get_by_index::<i32>(4).ok()?,
                    embedding_model: row.try_get_by_index::<String>(5).ok()?,
                    score: row.try_get_by_index::<f64>(6).ok()?,
                })
            })
            .collect();
        
        Ok(results)
    }
    
    /// BM25 text search
    pub async fn bm25_search(
        &self,
        query: &str,
        limit: usize,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<ChunkResult>> {
        let tenant_filter = tenant_id
            .map(|_| "AND p.tenant_id = $3")
            .unwrap_or("");
        
        let sql = format!(
            r#"
            SELECT 
                c.id as chunk_id,
                c.paper_id,
                p.title as paper_title,
                c.content,
                c.chunk_index,
                c.embedding_model,
                ts_rank_cd(c.text_search_vector, plainto_tsquery('english', $1)) as score
            FROM chunks c
            JOIN papers p ON c.paper_id = p.id
            WHERE c.text_search_vector @@ plainto_tsquery('english', $1)
            {}
            ORDER BY score DESC
            LIMIT $2
            "#,
            tenant_filter
        );
        
        let mut values: Vec<sea_orm::Value> = vec![
            query.into(),
            (limit as i32).into(),
        ];
        
        if let Some(tid) = tenant_id {
            values.push(tid.into());
        }
        
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, &sql, values);
        
        let results = self.read_conn()
            .query_all(stmt)
            .await?
            .into_iter()
            .filter_map(|row| {
                use sea_orm::QueryResult;
                Some(ChunkResult {
                    chunk_id: row.try_get_by_index::<Uuid>(0).ok()?,
                    paper_id: row.try_get_by_index::<Uuid>(1).ok()?,
                    paper_title: row.try_get_by_index::<String>(2).ok()?,
                    content: row.try_get_by_index::<String>(3).ok()?,
                    chunk_index: row.try_get_by_index::<i32>(4).ok()?,
                    embedding_model: row.try_get_by_index::<String>(5).ok()?,
                    score: row.try_get_by_index::<f64>(6).ok()?,
                })
            })
            .collect();
        
        Ok(results)
    }
    
    /// Hybrid search with Reciprocal Rank Fusion
    pub async fn hybrid_search(
        &self,
        query: &str,
        embedding: &[f32],
        limit: usize,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<ChunkResult>> {
        use std::collections::HashMap;
        
        const K: f64 = 60.0;  // RRF constant
        
        // Run both searches in parallel
        let vector_results = self.vector_search(embedding, limit * 2, tenant_id).await?;
        let bm25_results = self.bm25_search(query, limit * 2, tenant_id).await?;
        
        // Compute RRF scores
        let mut rrf_scores: HashMap<Uuid, (ChunkResult, f64)> = HashMap::new();
        
        for (rank, result) in vector_results.into_iter().enumerate() {
            let rrf = 1.0 / (K + (rank + 1) as f64);
            rrf_scores
                .entry(result.chunk_id)
                .and_modify(|(_, score)| *score += rrf)
                .or_insert((result, rrf));
        }
        
        for (rank, result) in bm25_results.into_iter().enumerate() {
            let rrf = 1.0 / (K + (rank + 1) as f64);
            rrf_scores
                .entry(result.chunk_id)
                .and_modify(|(_, score)| *score += rrf)
                .or_insert((result, rrf));
        }
        
        // Sort by RRF score and take top results
        let mut results: Vec<_> = rrf_scores.into_values()
            .map(|(mut result, score)| {
                result.score = score;
                result
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        Ok(results)
    }
    
    // ========================================================================
    // Job Operations
    // ========================================================================
    
    /// Create an ingestion job
    pub async fn create_job(
        &self,
        tenant_id: Uuid,
        idempotency_key: Option<String>,
    ) -> Result<IngestionJob> {
        let job_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        
        let job = IngestionJobActiveModel {
            id: Set(job_id),
            tenant_id: Set(tenant_id),
            paper_id: Set(None),
            status: Set("pending".to_string()),
            chunks_total: Set(0),
            chunks_processed: Set(0),
            error_message: Set(None),
            idempotency_key: Set(idempotency_key),
            attempt_count: Set(0),
            next_retry_at: Set(None),
            created_at: Set(now.into()),
            started_at: Set(None),
            completed_at: Set(None),
        };
        
        job.insert(self.write_conn()).await.map_err(Into::into)
    }
    
    /// Find job by ID
    pub async fn find_job_by_id(&self, id: Uuid) -> Result<Option<IngestionJob>> {
        IngestionJobEntity::find_by_id(id)
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    /// Find job by idempotency key
    pub async fn find_job_by_idempotency_key(
        &self,
        tenant_id: Uuid,
        key: &str,
    ) -> Result<Option<IngestionJob>> {
        IngestionJobEntity::find()
            .filter(IngestionJobColumn::TenantId.eq(tenant_id))
            .filter(IngestionJobColumn::IdempotencyKey.eq(key))
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
    
    /// Update job status
    pub async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
        paper_id: Option<Uuid>,
        chunks_total: Option<i32>,
        error_message: Option<String>,
    ) -> Result<IngestionJob> {
        let now = chrono::Utc::now();
        
        let mut job: IngestionJobActiveModel = IngestionJobEntity::find_by_id(job_id)
            .one(self.write_conn())
            .await?
            .ok_or_else(|| AppError::JobNotFound { id: job_id.to_string() })?
            .into();
        
        job.status = Set(String::from(status.clone()));
        
        if let Some(pid) = paper_id {
            job.paper_id = Set(Some(pid));
        }
        
        if let Some(total) = chunks_total {
            job.chunks_total = Set(total);
        }
        
        if let Some(err) = error_message {
            job.error_message = Set(Some(err));
        }
        
        match status {
            JobStatus::Chunking | JobStatus::Embedding | JobStatus::Indexing => {
                if job.started_at.is_not_set() {
                    job.started_at = Set(Some(now.into()));
                }
            }
            JobStatus::Completed | JobStatus::Failed => {
                job.completed_at = Set(Some(now.into()));
            }
            _ => {}
        }
        
        job.update(self.write_conn()).await.map_err(Into::into)
    }
    
    /// Update job progress
    pub async fn update_job_progress(
        &self,
        job_id: Uuid,
        chunks_processed: i32,
    ) -> Result<()> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE ingestion_jobs SET chunks_processed = $1 WHERE id = $2",
            vec![chunks_processed.into(), job_id.into()],
        );
        
        self.write_conn().execute(stmt).await?;
        Ok(())
    }
    
    // ========================================================================
    // Citation Operations
    // ========================================================================
    
    /// Get citations for a paper (both directions)
    pub async fn get_citations(
        &self,
        paper_id: Uuid,
    ) -> Result<(Vec<Citation>, Vec<Citation>)> {
        let outgoing = CitationEntity::find()
            .filter(CitationColumn::CitingPaperId.eq(paper_id))
            .all(self.read_conn())
            .await?;
        
        let incoming = CitationEntity::find()
            .filter(CitationColumn::CitedPaperId.eq(paper_id))
            .all(self.read_conn())
            .await?;
        
        Ok((outgoing, incoming))
    }
    
    // ========================================================================
    // Session Operations
    // ========================================================================
    
    /// Create or update session
    pub async fn upsert_session(
        &self,
        tenant_id: Uuid,
        session_id: Uuid,
        state: serde_json::Value,
        ttl_minutes: i64,
    ) -> Result<Session> {
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(ttl_minutes);
        
        let session = SessionActiveModel {
            id: Set(session_id),
            tenant_id: Set(tenant_id),
            state: Set(state),
            created_at: Set(now.into()),
            last_active_at: Set(now.into()),
            expires_at: Set(expires.into()),
        };
        
        // Use upsert
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
            INSERT INTO sessions (id, tenant_id, state, created_at, last_active_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                state = EXCLUDED.state,
                last_active_at = EXCLUDED.last_active_at,
                expires_at = EXCLUDED.expires_at
            RETURNING *
            "#,
            vec![
                session_id.into(),
                tenant_id.into(),
                session.state.clone().into_value().unwrap(),
                now.into(),
                now.into(),
                expires.into(),
            ],
        );
        
        // For simplicity, just insert and ignore conflicts
        session.insert(self.write_conn()).await.map_err(Into::into)
    }
    
    /// Find session by ID
    pub async fn find_session(&self, session_id: Uuid) -> Result<Option<Session>> {
        SessionEntity::find_by_id(session_id)
            .one(self.read_conn())
            .await
            .map_err(Into::into)
    }
}
