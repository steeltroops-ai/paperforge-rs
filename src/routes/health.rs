//! Health check endpoints for liveness and readiness probes
//! 
//! - `/health` - Basic liveness check (always returns OK if app is running)
//! - `/readiness` - Deep readiness check (verifies database connectivity)

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use crate::db::Repository;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub database: CheckResult,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub status: &'static str,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Liveness probe - always returns OK if the app is running
/// 
/// Used by load balancers and orchestrators to verify the process is alive.
/// Does NOT check dependencies - use /readiness for that.
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness probe - verifies all dependencies are accessible
/// 
/// Used by orchestrators to determine if the service can accept traffic.
/// Checks:
/// - Database connectivity (with latency measurement)
/// 
/// Returns 503 if any check fails.
pub async fn readiness_check(
    State(repo): State<Repository>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    
    let db_check = match repo.ping().await {
        Ok(_) => CheckResult {
            status: "healthy",
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => CheckResult {
            status: "unhealthy",
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    };
    
    let overall_healthy = db_check.status == "healthy";
    
    let response = ReadinessResponse {
        status: if overall_healthy { "ready" } else { "not_ready" },
        version: env!("CARGO_PKG_VERSION"),
        checks: HealthChecks {
            database: db_check,
        },
    };
    
    if overall_healthy {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}
