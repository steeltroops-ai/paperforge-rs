//! Health check handlers

use axum::{extract::State, Json};
use serde::Serialize;
use crate::AppState;
use paperforge_common::errors::Result;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Serialize)]
pub struct ReadyResponse {
    pub status: String,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: CheckResult,
}

#[derive(Serialize)]
pub struct CheckResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Liveness probe - always returns healthy if server is running
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
    })
}

/// Readiness probe - checks all dependencies
pub async fn ready(State(state): State<AppState>) -> Json<ReadyResponse> {
    let start = std::time::Instant::now();
    
    let db_check = match state.db.ping().await {
        Ok(_) => CheckResult {
            status: "up".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => CheckResult {
            status: "down".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
    };
    
    let all_healthy = db_check.status == "up";
    
    Json(ReadyResponse {
        status: if all_healthy { "ready" } else { "not_ready" }.to_string(),
        checks: HealthChecks {
            database: db_check,
        },
    })
}
