//! Metrics and observability utilities
//!
//! Provides Prometheus metrics with SLO-aligned histograms
//! and standardized naming conventions.

use metrics::{
    counter, describe_counter, describe_gauge, describe_histogram, 
    gauge, histogram, Counter, Gauge, Histogram, Unit,
};
use std::time::Instant;

/// Metrics prefix for all PaperForge metrics  
pub const METRICS_PREFIX: &str = "paperforge";

/// SLO-aligned histogram buckets for request latency (in seconds)
/// Targets: P50 < 50ms, P99 < 150ms
pub const LATENCY_BUCKETS: &[f64] = &[
    0.001,  // 1ms
    0.005,  // 5ms
    0.010,  // 10ms
    0.025,  // 25ms
    0.050,  // 50ms - P50 target
    0.075,  // 75ms
    0.100,  // 100ms
    0.150,  // 150ms - P99 target
    0.250,  // 250ms
    0.500,  // 500ms
    1.000,  // 1s
    2.500,  // 2.5s
    5.000,  // 5s
    10.00,  // 10s
];

/// Buckets for embedding latency (typically slower)
pub const EMBEDDING_BUCKETS: &[f64] = &[
    0.050,  // 50ms
    0.100,  // 100ms
    0.250,  // 250ms
    0.500,  // 500ms
    1.000,  // 1s
    2.000,  // 2s
    5.000,  // 5s
    10.00,  // 10s
    30.00,  // 30s
];

/// Register all metric descriptions
pub fn register_metrics() {
    // Request metrics
    describe_counter!(
        format!("{}_requests_total", METRICS_PREFIX),
        Unit::Count,
        "Total number of HTTP requests"
    );
    
    describe_histogram!(
        format!("{}_request_duration_seconds", METRICS_PREFIX),
        Unit::Seconds,
        "HTTP request latency in seconds"
    );
    
    // Search metrics
    describe_counter!(
        format!("{}_search_queries_total", METRICS_PREFIX),
        Unit::Count,
        "Total number of search queries"
    );
    
    describe_histogram!(
        format!("{}_search_duration_seconds", METRICS_PREFIX),
        Unit::Seconds,
        "Search query latency in seconds"
    );
    
    describe_gauge!(
        format!("{}_search_results_count", METRICS_PREFIX),
        Unit::Count,
        "Number of results returned from search"
    );
    
    // Ingestion metrics
    describe_counter!(
        format!("{}_papers_ingested_total", METRICS_PREFIX),
        Unit::Count,
        "Total papers ingested"
    );
    
    describe_counter!(
        format!("{}_chunks_created_total", METRICS_PREFIX),
        Unit::Count,
        "Total chunks created"
    );
    
    describe_histogram!(
        format!("{}_ingestion_duration_seconds", METRICS_PREFIX),
        Unit::Seconds,
        "Paper ingestion latency in seconds"
    );
    
    // Embedding metrics
    describe_counter!(
        format!("{}_embedding_requests_total", METRICS_PREFIX),
        Unit::Count,
        "Total embedding API requests"
    );
    
    describe_histogram!(
        format!("{}_embedding_duration_seconds", METRICS_PREFIX),
        Unit::Seconds,
        "Embedding generation latency in seconds"
    );
    
    describe_counter!(
        format!("{}_embedding_errors_total", METRICS_PREFIX),
        Unit::Count,
        "Total embedding API errors"
    );
    
    // Database metrics
    describe_gauge!(
        format!("{}_db_connections_active", METRICS_PREFIX),
        Unit::Count,
        "Active database connections"
    );
    
    describe_gauge!(
        format!("{}_db_connections_idle", METRICS_PREFIX),
        Unit::Count,
        "Idle database connections"
    );
    
    describe_histogram!(
        format!("{}_db_query_duration_seconds", METRICS_PREFIX),
        Unit::Seconds,
        "Database query latency in seconds"
    );
    
    // Queue metrics  
    describe_gauge!(
        format!("{}_queue_depth", METRICS_PREFIX),
        Unit::Count,
        "Number of messages in queue"
    );
    
    describe_counter!(
        format!("{}_queue_messages_processed_total", METRICS_PREFIX),
        Unit::Count,
        "Total queue messages processed"
    );
    
    // Cache metrics
    describe_counter!(
        format!("{}_cache_hits_total", METRICS_PREFIX),
        Unit::Count,
        "Total cache hits"
    );
    
    describe_counter!(
        format!("{}_cache_misses_total", METRICS_PREFIX),
        Unit::Count,
        "Total cache misses"
    );
    
    tracing::info!("Metrics registered");
}

/// Helper to record request metrics
pub struct RequestMetrics {
    start: Instant,
    endpoint: String,
    method: String,
}

impl RequestMetrics {
    /// Start tracking a request
    pub fn start(method: &str, endpoint: &str) -> Self {
        Self {
            start: Instant::now(),
            endpoint: endpoint.to_string(),
            method: method.to_string(),
        }
    }
    
    /// Record request completion
    pub fn finish(self, status: u16) {
        let duration = self.start.elapsed().as_secs_f64();
        
        counter!(
            format!("{}_requests_total", METRICS_PREFIX),
            "method" => self.method.clone(),
            "endpoint" => self.endpoint.clone(),
            "status" => status.to_string()
        )
        .increment(1);
        
        histogram!(
            format!("{}_request_duration_seconds", METRICS_PREFIX),
            "method" => self.method,
            "endpoint" => self.endpoint
        )
        .record(duration);
    }
}

/// Helper to record search metrics
pub fn record_search(duration_secs: f64, mode: &str, result_count: usize) {
    counter!(
        format!("{}_search_queries_total", METRICS_PREFIX),
        "mode" => mode.to_string()
    )
    .increment(1);
    
    histogram!(
        format!("{}_search_duration_seconds", METRICS_PREFIX),
        "mode" => mode.to_string()
    )
    .record(duration_secs);
    
    gauge!(
        format!("{}_search_results_count", METRICS_PREFIX),
        "mode" => mode.to_string()
    )
    .set(result_count as f64);
}

/// Helper to record embedding metrics
pub fn record_embedding(duration_secs: f64, model: &str, batch_size: usize, success: bool) {
    let status = if success { "success" } else { "error" };
    
    counter!(
        format!("{}_embedding_requests_total", METRICS_PREFIX),
        "model" => model.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
    
    if success {
        histogram!(
            format!("{}_embedding_duration_seconds", METRICS_PREFIX),
            "model" => model.to_string()
        )
        .record(duration_secs);
    } else {
        counter!(
            format!("{}_embedding_errors_total", METRICS_PREFIX),
            "model" => model.to_string()
        )
        .increment(1);
    }
}

/// Helper to record cache metrics
pub fn record_cache(hit: bool, cache_name: &str) {
    if hit {
        counter!(
            format!("{}_cache_hits_total", METRICS_PREFIX),
            "cache" => cache_name.to_string()
        )
        .increment(1);
    } else {
        counter!(
            format!("{}_cache_misses_total", METRICS_PREFIX),
            "cache" => cache_name.to_string()
        )
        .increment(1);
    }
}

/// Helper to record ingestion metrics
pub fn record_ingestion(duration_secs: f64, chunks_created: usize, tenant_id: &str) {
    counter!(
        format!("{}_papers_ingested_total", METRICS_PREFIX),
        "tenant" => tenant_id.to_string()
    )
    .increment(1);
    
    counter!(
        format!("{}_chunks_created_total", METRICS_PREFIX),
        "tenant" => tenant_id.to_string()
    )
    .increment(chunks_created as u64);
    
    histogram!(
        format!("{}_ingestion_duration_seconds", METRICS_PREFIX)
    )
    .record(duration_secs);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_latency_buckets() {
        // Verify buckets are sorted and contain SLO targets
        let mut prev = 0.0;
        for &bucket in LATENCY_BUCKETS {
            assert!(bucket > prev);
            prev = bucket;
        }
        
        // P50 target (50ms) should be in buckets
        assert!(LATENCY_BUCKETS.contains(&0.050));
        // P99 target (150ms) should be in buckets
        assert!(LATENCY_BUCKETS.contains(&0.150));
    }
    
    #[test]
    fn test_request_metrics() {
        let metrics = RequestMetrics::start("GET", "/v2/search");
        std::thread::sleep(std::time::Duration::from_millis(10));
        metrics.finish(200);
        // Just verify it runs without panic
    }
}
