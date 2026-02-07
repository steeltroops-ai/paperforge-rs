use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::services::AppState;
use crate::errors::AppError;
use tracing::instrument;
use crate::services::search::SearchResult;

/// Search query parameters with validation
#[derive(Debug, Deserialize, Validate)]
pub struct SearchParams {
    /// Search query string (required, 1-1000 chars)
    #[validate(length(min = 1, max = 1000, message = "Query must be 1-1000 characters"))]
    q: String,
    
    /// Maximum number of results (default: 10, max: 50)
    #[validate(range(min = 1, max = 50, message = "Limit must be 1-50"))]
    limit: Option<u64>,
    
    /// Enable hybrid search (vector + text). Default: true
    hybrid: Option<bool>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query: String,
    pub total_results: usize,
    pub hybrid_search: bool,
}

/// Search papers endpoint
/// 
/// # Query Parameters
/// - `q` (required): Search query string (1-1000 characters)
/// - `limit` (optional): Maximum results to return (1-50, default: 10)
/// - `hybrid` (optional): Use hybrid search (default: true)
/// 
/// # Returns
/// Ranked list of matching chunks with similarity scores
#[instrument(
    skip(state),
    fields(
        query_len = params.q.len(),
        limit = params.limit,
        hybrid = params.hybrid,
    )
)]
pub async fn search_papers(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, AppError> {
    // Validate parameters
    params.validate().map_err(|e| {
        let messages: Vec<String> = e.field_errors()
            .into_iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |err| {
                    format!("{}: {}", field, err.message.as_ref().map(|m| m.to_string()).unwrap_or_default())
                })
            })
            .collect();
        AppError::ValidationError(messages.join("; "))
    })?;
    
    // Additional validation
    if params.q.trim().is_empty() {
        return Err(AppError::MissingField("q".to_string()));
    }

    let limit = params.limit.unwrap_or(10).min(50);
    let hybrid = params.hybrid.unwrap_or(true); // Default to hybrid for better results
    let query = params.q.clone();

    let results = state.search_service.query(params.q, limit, hybrid).await?;
    let total_results = results.len();

    Ok(Json(SearchResponse { 
        results,
        query,
        total_results,
        hybrid_search: hybrid,
    }))
}
