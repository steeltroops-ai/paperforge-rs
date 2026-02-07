use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::services::AppState;
use crate::errors::AppError;
use tracing::instrument;
use crate::services::search::SearchResult;

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
    limit: Option<u64>,
    hybrid: Option<bool>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    results: Vec<SearchResult>,
}

#[instrument(skip(state))]
pub async fn search_papers(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, AppError> {
    if params.q.trim().is_empty() {
        return Err(AppError::ValidationError("Query string cannot be empty".to_string()));
    }

    let limit = params.limit.unwrap_or(10).min(50); // Cap limit at 50
    let hybrid = params.hybrid.unwrap_or(false);

    let results = state.search_service.query(params.q, limit, hybrid).await?;

    Ok(Json(SearchResponse { results }))
}
