use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set, DbErr, Statement, ConnectionTrait, Value, DbBackend, FromQueryResult};
use uuid::Uuid;
use super::models::{self, Entity as PaperEntity, ChunkModel};
use crate::config::DatabaseConfig;

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
    pub created_at: chrono::DateTime<chrono::FixedOffset>, 
    pub distance: f64,
}

impl Repository {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, DbErr> {
        let mut opt = sea_orm::ConnectOptions::new(&config.url);
        opt.max_connections(config.max_connections)
           .min_connections(config.min_connections)
           .connect_timeout(std::time::Duration::from_secs(config.connect_timeout))
           .sqlx_logging(true);
        
        let db = sea_orm::Database::connect(opt).await?;
        Ok(Self { db })
    }

    pub async fn create_paper(&self, title: String, abstract_text: String, source: Option<String>) -> Result<Uuid, DbErr> {
        let id = Uuid::new_v4();
        let paper = models::ActiveModel {
            id: Set(id),
            title: Set(title),
            abstract_text: Set(abstract_text), // Note field name
            source: Set(source),
            published_at: Set(Some(chrono::Utc::now().into())),
            created_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        };
        
        paper.insert(&self.db).await?;
        Ok(id)
    }

    pub async fn create_chunks(&self, paper_id: Uuid, chunks: Vec<(i32, String, Vec<f32>, i32)>) -> Result<(), DbErr> {
        // Bulk insert using raw SQL because SeaORM entity/vector mapping is tricky
        // VALUES ($1, $2, $3, $4, $5::vector, $6)
        
        for (index, content, embedding, token_count) in chunks {
            // Convert Vec<f32> to string format "[1.0, 2.0, ...]" for SQL vector literal
            let embedding_str = format!("[{}]", embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));
            
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                INSERT INTO chunks (paper_id, chunk_index, content, embedding, token_count)
                VALUES ($1, $2, $3, $4::vector, $5)
                "#,
                vec![
                    paper_id.into(),
                    index.into(),
                    content.into(),
                    embedding_str.into(),
                    token_count.into(),
                ],
            );
            
            self.db.execute(stmt).await?;
        }
        
        Ok(())
    }

    /// Performs Hybrid Search (Vector + Full-Text)
    /// In this MVP implementation, we use a weighted scoring strategy:
    /// Score = (0.7 * Vector_Similarity) + (0.3 * Text_Rank)
    /// Vector_Similarity = 1 - (embedding <=> query_vec)
    /// Text_Rank = ts_rank(to_tsvector('english', content), plainto_tsquery('english', query_text))
    pub async fn search_hybrid(&self, query_text: String, query_embedding: Vec<f32>, limit: u64) -> Result<Vec<(ChunkModel, f64)>, DbErr> {
         let embedding_str = format!("[{}]", query_embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));
         
         // SQL: combines ts_rank and vector distance
         // Note: We protect against SQL injection via parameters, but `embedding_str` is constructed internally.
         // Usually we would prefer true RRF (Reciprocal Rank Fusion) but that requires complex SQL or application merging.
         // For MVP Advanced: Weighted Sum is standard.
         
         let sql = format!(
            r#"
            WITH semantic_search AS (
                SELECT id, paper_id, chunk_index, content, token_count, created_at, 
                       (1 - (embedding <=> '{}'::vector)) as vector_score,
                       ts_rank_cd(to_tsvector('english', content), plainto_tsquery('english', $1)) as text_score
                FROM chunks
                WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $1) 
                   OR (embedding <=> '{}'::vector) < 0.5 -- Limit vector candidates to reasonable similarty
            )
            SELECT *,
                   (0.7 * vector_score + 0.3 * text_score) as hybrid_score
            FROM semantic_search
            ORDER BY hybrid_score DESC
            LIMIT {}
            "#,
            embedding_str, embedding_str, limit
        );
        
        // We need a slightly different mapping logic since we return hybrid_score.
        // We reuse `SearchResult` but map `distance` = `1 - hybrid_score` just to keep interface consistent 
        // or update SearchResult to use score.
        // Let's create a new struct for clarity.
        
        #[derive(Debug, FromQueryResult)]
        struct HybridResult {
            id: Uuid,
            paper_id: Uuid,
            chunk_index: i32,
            content: String,
            token_count: i32,
            created_at: chrono::DateTime<chrono::FixedOffset>,
            hybrid_score: f64,
        }

        let results: Vec<HybridResult> = HybridResult::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres, 
                &sql, 
                vec![query_text.into()] // $1 is query text
            ))
            .all(&self.db)
            .await?;
            
        Ok(results.into_iter().map(|r| (
            models::ChunkModel {
                id: r.id,
                paper_id: r.paper_id,
                chunk_index: r.chunk_index,
                content: r.content,
                embedding: None, 
                embedding_json: None,
                token_count: r.token_count,
                created_at: r.created_at,
            },
            1.0 - r.hybrid_score // Map back to "distance" concept for now, or just use score directly in service
        )).collect())
    }
    
    // Kept for backward compat or pure vector search
    // We can also route this to hybrid if query text is provided
    pub async fn search_similar_chunks(&self, query_embedding: Vec<f32>, limit: u64) -> Result<Vec<(ChunkModel, f64)>, DbErr> {
        // Implementation from previous turn...
         let embedding_str = format!("[{}]", query_embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));

        // Use raw SQL to get distances
        let sql = format!(
            r#"
            SELECT id, paper_id, chunk_index, content, token_count, created_at, 
                   embedding <=> '{}'::vector as distance
            FROM chunks
            ORDER BY distance ASC
            LIMIT {}
            "#,
            embedding_str, limit
        );

        let results: Vec<SearchResult> = SearchResult::find_by_statement(Statement::from_string(DbBackend::Postgres, sql))
            .all(&self.db)
            .await?;
            
        Ok(results.into_iter().map(|r| (
            models::ChunkModel {
                id: r.id,
                paper_id: r.paper_id,
                chunk_index: r.chunk_index,
                content: r.content,
                embedding: None, 
                embedding_json: None,
                token_count: r.token_count,
                created_at: r.created_at,
            },
            r.distance
        )).collect())
    }
    
    pub async fn get_paper(&self, id: Uuid) -> Result<Option<models::Model>, DbErr> {
        models::Entity::find_by_id(id).one(&self.db).await
    }
}
