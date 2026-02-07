use axum::{
    routing::get,
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use metrics::{describe_counter, describe_histogram, describe_gauge};

/// Setup Prometheus metrics collection with proper descriptions
/// 
/// Metrics exposed:
/// - Counters: Total requests, errors, papers ingested, chunks created, searches
/// - Histograms: Request duration, embedding duration, search duration
/// - Gauges: Active connections, queue depth
pub fn setup_metrics() -> (PrometheusMetricLayer, Router) {
    // Register metric descriptions for Prometheus
    register_metric_descriptions();
    
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    
    let app = Router::new()
        .route("/metrics", get(|| async move { metric_handle.render() }));
    
    (prometheus_layer, app)
}

/// Register all metric descriptions with units
fn register_metric_descriptions() {
    // =========================================================================
    // COUNTERS
    // =========================================================================
    
    describe_counter!(
        "paperforge_ingest_papers_total",
        "Total number of papers successfully ingested"
    );
    
    describe_counter!(
        "paperforge_ingest_chunks_total",
        "Total number of chunks created from paper ingestion"
    );
    
    describe_counter!(
        "paperforge_search_ops_total",
        "Total number of search operations performed"
    );
    
    describe_counter!(
        "paperforge_errors_total",
        "Total number of errors by type"
    );
    
    describe_counter!(
        "paperforge_embedding_requests_total",
        "Total number of embedding API requests"
    );
    
    describe_counter!(
        "paperforge_embedding_retries_total",
        "Total number of embedding API request retries"
    );
    
    // =========================================================================
    // HISTOGRAMS - SLO-aligned buckets
    // =========================================================================
    
    // Ingest duration - target SLO: P99 < 5s
    describe_histogram!(
        "paperforge_ingest_duration_seconds",
        metrics::Unit::Seconds,
        "Time to ingest a paper including embedding generation"
    );
    
    // Embedding duration - target SLO: P99 < 2s per chunk
    describe_histogram!(
        "paperforge_embedding_duration_seconds",
        metrics::Unit::Seconds,
        "Time to generate embeddings for paper chunks"
    );
    
    // Search duration - target SLO: P99 < 200ms
    describe_histogram!(
        "paperforge_search_duration_seconds",
        metrics::Unit::Seconds,
        "Time to execute a search query"
    );
    
    // Query embedding duration - target SLO: P99 < 100ms
    describe_histogram!(
        "paperforge_query_embedding_duration_seconds",
        metrics::Unit::Seconds,
        "Time to generate embedding for search query"
    );
    
    // Search results count distribution
    describe_histogram!(
        "paperforge_search_results_count",
        "Number of results returned per search query"
    );
    
    // Request size distribution
    describe_histogram!(
        "paperforge_request_body_bytes",
        metrics::Unit::Bytes,
        "Request body size distribution"
    );
    
    // =========================================================================
    // GAUGES
    // =========================================================================
    
    describe_gauge!(
        "paperforge_db_connections_active",
        "Number of active database connections"
    );
    
    describe_gauge!(
        "paperforge_db_connections_idle",
        "Number of idle database connections"
    );
}

/// Record an error with type label
pub fn record_error(error_type: &str) {
    metrics::counter!(
        "paperforge_errors_total",
        "error_type" => error_type.to_string()
    ).increment(1);
}

/// SLO threshold constants for alerting reference
pub mod slo {
    /// Target P99 latency for search requests
    pub const SEARCH_P99_MS: u64 = 200;
    
    /// Target P99 latency for ingest requests
    pub const INGEST_P99_MS: u64 = 5000;
    
    /// Target error rate percentage
    pub const ERROR_RATE_PERCENT: f64 = 1.0;
    
    /// Availability target
    pub const AVAILABILITY_PERCENT: f64 = 99.9;
}
