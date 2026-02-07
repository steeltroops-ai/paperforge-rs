pub mod ingest;
pub mod search;
pub mod health;

use axum::Router;
use axum::routing::{get, post};
use axum::middleware;
use tower::ServiceBuilder;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::request_id::{MakeRequestId, RequestId, SetRequestIdLayer, PropagateRequestIdLayer};
use std::time::Duration;
use crate::services::AppState;
use crate::db::Repository;
use crate::metrics;
use crate::middleware as app_middleware;

/// Maximum concurrent requests (backpressure control)
const MAX_CONCURRENT_REQUESTS: usize = 100;

/// Request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub fn create_router(state: AppState, repo: Repository) -> Router {
    let (prometheus_layer, metrics_router) = metrics::setup_metrics();

    // Health routes (no auth required)
    let health_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/readiness", get(health::readiness_check))
        .with_state(repo);

    // API routes (with auth)
    let api_routes = Router::new()
        .route("/ingest", post(ingest::ingest_paper))
        .route("/search", get(search::search_papers))
        .with_state(state);

    // Build the router with middleware stack
    Router::new()
        .merge(api_routes)
        .merge(health_routes)
        .merge(metrics_router)
        .layer(
            ServiceBuilder::new()
                // Prometheus metrics (outermost - captures all requests)
                .layer(prometheus_layer)
                // Request timeout
                .layer(TimeoutLayer::new(Duration::from_secs(REQUEST_TIMEOUT_SECS)))
                // Concurrency limit for backpressure
                .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_REQUESTS))
                // Request ID propagation
                .layer(axum::middleware::from_fn(app_middleware::request_id))
                // Content-Length limit
                .layer(axum::middleware::from_fn(app_middleware::content_length_limit))
                // API key authentication
                .layer(axum::middleware::from_fn(app_middleware::api_key_auth))
        )
}
