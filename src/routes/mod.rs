pub mod ingest;
pub mod search;

use axum::Router;
use axum::routing::{get, post};
use crate::services::AppState;
use crate::metrics;

pub fn create_router(state: AppState) -> Router {
    let (prometheus_layer, metrics_router) = metrics::setup_metrics();

    let api_routes = Router::new()
        .route("/ingest", post(ingest::ingest_paper))
        .route("/search", get(search::search_papers))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    Router::new()
        .merge(api_routes)
        .merge(metrics_router)
        .layer(prometheus_layer)
}
