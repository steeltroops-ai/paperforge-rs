//! gRPC service implementation for search

use crate::retrieval::{HybridRetriever, BM25Retriever, VectorRetriever, Retriever, SearchRequest, RetrievalMode};
use crate::citation::{CitationGraph, PageRankScorer, PageRankConfig};
use paperforge_common::db::DbPool;
use paperforge_common::cache::{Cache, CacheConfig};
use paperforge_common::proto::search::{
    search_service_server::{SearchService, SearchServiceServer},
    SearchRequest as ProtoSearchRequest,
    SearchResponse as ProtoSearchResponse,
    SearchResult as ProtoSearchResult,
    SearchMode,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// Search gRPC service
pub struct SearchGrpcService {
    db: Arc<DbPool>,
    cache: Option<Arc<Cache>>,
    vector: VectorRetriever,
    bm25: BM25Retriever,
    hybrid: HybridRetriever,
}

impl SearchGrpcService {
    /// Create a new search service
    pub fn new(db: Arc<DbPool>, cache: Option<Arc<Cache>>) -> Self {
        Self {
            db: db.clone(),
            cache,
            vector: VectorRetriever::new(db.clone()),
            bm25: BM25Retriever::new(db.clone()),
            hybrid: HybridRetriever::new(db),
        }
    }
    
    /// Create the gRPC server
    pub fn into_server(self) -> SearchServiceServer<Self> {
        SearchServiceServer::new(self)
    }
    
    /// Convert proto mode to internal mode
    fn convert_mode(mode: i32) -> RetrievalMode {
        match SearchMode::try_from(mode) {
            Ok(SearchMode::Vector) => RetrievalMode::Vector,
            Ok(SearchMode::Bm25) => RetrievalMode::BM25,
            Ok(SearchMode::Hybrid) | _ => RetrievalMode::Hybrid,
        }
    }
    
    /// Generate cache key for search
    fn cache_key(&self, req: &ProtoSearchRequest) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&req.query);
        hasher.update(req.mode.to_le_bytes());
        hasher.update(req.limit.to_le_bytes());
        let hash = hex::encode(hasher.finalize());
        format!("search:{}:{}:{}", req.tenant_id, req.mode, &hash[..16])
    }
}

#[tonic::async_trait]
impl SearchService for SearchGrpcService {
    async fn search(
        &self,
        request: Request<ProtoSearchRequest>,
    ) -> Result<Response<ProtoSearchResponse>, Status> {
        let req = request.into_inner();
        let start = std::time::Instant::now();
        
        // Parse tenant ID
        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;
        
        // Check cache first
        let cache_key = self.cache_key(&req);
        if let Some(cache) = &self.cache {
            if let Ok(Some(cached)) = cache.get::<ProtoSearchResponse>(&cache_key).await {
                tracing::debug!(cache_key = %cache_key, "Cache hit");
                return Ok(Response::new(cached));
            }
        }
        
        // Build search request
        let mode = Self::convert_mode(req.mode);
        let search_req = SearchRequest {
            tenant_id,
            query: req.query.clone(),
            query_embedding: if req.query_embedding.is_empty() {
                None
            } else {
                Some(req.query_embedding.clone())
            },
            mode,
            limit: req.limit as usize,
            min_score: if req.min_score > 0.0 { Some(req.min_score) } else { None },
            paper_ids: None,
        };
        
        // Execute search
        let chunks = match mode {
            RetrievalMode::Vector => self.vector.retrieve(&search_req).await,
            RetrievalMode::BM25 => self.bm25.retrieve(&search_req).await,
            RetrievalMode::Hybrid => self.hybrid.retrieve(&search_req).await,
        }.map_err(|e| Status::internal(format!("Search failed: {}", e)))?;
        
        // Convert to proto
        let results: Vec<ProtoSearchResult> = chunks.iter().map(|c| {
            ProtoSearchResult {
                chunk_id: c.chunk_id.to_string(),
                paper_id: c.paper_id.to_string(),
                paper_title: c.paper_title.clone(),
                content: c.content.clone(),
                chunk_index: c.chunk_index,
                score: c.score,
            }
        }).collect();
        
        let response = ProtoSearchResponse {
            results,
            total_count: chunks.len() as u32,
            query_time_ms: start.elapsed().as_millis() as u64,
            mode: req.mode,
        };
        
        // Cache the result
        if let Some(cache) = &self.cache {
            let _ = cache.set_with_ttl(&cache_key, &response, 300).await;
        }
        
        Ok(Response::new(response))
    }
}
