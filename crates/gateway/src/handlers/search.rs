//! Search handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;
use validator::Validate;

use crate::AppState;
use paperforge_common::{
    auth::AuthContext,
    db::{ChunkResult, Repository},
    errors::{AppError, Result},
    metrics,
};

/// Search request
#[derive(Debug, Deserialize, Validate)]
pub struct SearchRequest {
    #[validate(length(min = 1, max = 1000))]
    pub query: String,
    
    #[serde(default)]
    pub options: SearchOptions,
}

#[derive(Debug, Default, Deserialize)]
pub struct SearchOptions {
    /// Search mode: vector, bm25, hybrid (default)
    #[serde(default = "default_mode")]
    pub mode: String,
    
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    
    /// Offset for pagination
    #[serde(default)]
    pub offset: usize,
    
    /// Enable reranking
    #[serde(default)]
    pub rerank: bool,
    
    /// Minimum score threshold
    #[serde(default)]
    pub min_score: Option<f64>,
    
    /// Filters
    #[serde(default)]
    pub filters: SearchFilters,
}

#[derive(Debug, Default, Deserialize)]
pub struct SearchFilters {
    pub source: Option<Vec<String>>,
    pub published_after: Option<String>,
    pub published_before: Option<String>,
}

fn default_mode() -> String { "hybrid".to_string() }
fn default_limit() -> usize { 20 }

/// Search response
#[derive(Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub mode: String,
    pub total_results: usize,
    pub results: Vec<SearchResultItem>,
    pub processing_time_ms: u64,
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub chunk_id: Uuid,
    pub paper_id: Uuid,
    pub paper_title: String,
    pub content: String,
    pub chunk_index: i32,
    pub score: f64,
}

/// Batch search request
#[derive(Debug, Deserialize)]
pub struct BatchSearchRequest {
    pub queries: Vec<SingleQuery>,
    #[serde(default)]
    pub options: SearchOptions,
}

#[derive(Debug, Deserialize)]
pub struct SingleQuery {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Batch search response
#[derive(Serialize)]
pub struct BatchSearchResponse {
    pub results: Vec<BatchSearchResult>,
    pub processing_time_ms: u64,
}

#[derive(Serialize)]
pub struct BatchSearchResult {
    pub query: String,
    pub results: Vec<SearchResultItem>,
}

/// Perform a search
pub async fn search(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>> {
    let start = Instant::now();
    
    request.validate().map_err(|e| AppError::Validation {
        message: e.to_string(),
        field: None,
    })?;
    
    let repo = Repository::new(state.db.clone());
    
    // Get embedding for the query (TODO: use actual embedder)
    // For now, using mock embedding
    let mock_embedding: Vec<f32> = (0..768).map(|i| (i as f32).sin()).collect();
    
    let results = match request.options.mode.as_str() {
        "vector" => {
            repo.vector_search(&mock_embedding, request.options.limit, Some(auth.tenant_id)).await?
        }
        "bm25" => {
            repo.bm25_search(&request.query, request.options.limit, Some(auth.tenant_id)).await?
        }
        "hybrid" | _ => {
            repo.hybrid_search(&request.query, &mock_embedding, request.options.limit, Some(auth.tenant_id)).await?
        }
    };
    
    // Apply min_score filter if specified
    let results: Vec<_> = if let Some(min_score) = request.options.min_score {
        results.into_iter()
            .filter(|r| r.score >= min_score)
            .collect()
    } else {
        results
    };
    
    let processing_time_ms = start.elapsed().as_millis() as u64;
    
    // Record metrics
    metrics::record_search(
        processing_time_ms as f64 / 1000.0,
        &request.options.mode,
        results.len(),
    );
    
    tracing::info!(
        query = %request.query,
        mode = %request.options.mode,
        results = results.len(),
        latency_ms = processing_time_ms,
        tenant_id = %auth.tenant_id,
        "Search completed"
    );
    
    Ok(Json(SearchResponse {
        query: request.query,
        mode: request.options.mode,
        total_results: results.len(),
        results: results.into_iter().map(|r| SearchResultItem {
            chunk_id: r.chunk_id,
            paper_id: r.paper_id,
            paper_title: r.paper_title,
            content: r.content,
            chunk_index: r.chunk_index,
            score: r.score,
        }).collect(),
        processing_time_ms,
    }))
}

/// Batch search for multiple queries
pub async fn batch_search(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<BatchSearchRequest>,
) -> Result<Json<BatchSearchResponse>> {
    let start = Instant::now();
    
    if request.queries.len() > 10 {
        return Err(AppError::Validation {
            message: "Maximum 10 queries per batch".to_string(),
            field: Some("queries".to_string()),
        });
    }
    
    let repo = Repository::new(state.db.clone());
    let mut batch_results = Vec::with_capacity(request.queries.len());
    
    for single in request.queries {
        // Mock embedding for each query
        let mock_embedding: Vec<f32> = (0..768).map(|i| (i as f32).sin()).collect();
        
        let results = match request.options.mode.as_str() {
            "vector" => {
                repo.vector_search(&mock_embedding, single.limit, Some(auth.tenant_id)).await?
            }
            "bm25" => {
                repo.bm25_search(&single.query, single.limit, Some(auth.tenant_id)).await?
            }
            "hybrid" | _ => {
                repo.hybrid_search(&single.query, &mock_embedding, single.limit, Some(auth.tenant_id)).await?
            }
        };
        
        batch_results.push(BatchSearchResult {
            query: single.query,
            results: results.into_iter().map(|r| SearchResultItem {
                chunk_id: r.chunk_id,
                paper_id: r.paper_id,
                paper_title: r.paper_title,
                content: r.content,
                chunk_index: r.chunk_index,
                score: r.score,
            }).collect(),
        });
    }
    
    let processing_time_ms = start.elapsed().as_millis() as u64;
    
    Ok(Json(BatchSearchResponse {
        results: batch_results,
        processing_time_ms,
    }))
}
