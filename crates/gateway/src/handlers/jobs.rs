//! Job status handlers

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;
use paperforge_common::{
    auth::AuthContext,
    db::Repository,
    errors::{AppError, Result},
};

/// Job status response
#[derive(Serialize)]
pub struct JobResponse {
    pub job_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_id: Option<Uuid>,
    pub chunks_created: i32,
    pub chunks_total: i32,
    pub progress_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub created_at: String,
}

/// Get job status
pub async fn get_job(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobResponse>> {
    let repo = Repository::new(state.db.clone());
    
    let job = repo.find_job_by_id(job_id)
        .await?
        .ok_or_else(|| AppError::JobNotFound { 
            id: job_id.to_string() 
        })?;
    
    // Verify tenant access
    if job.tenant_id != auth.tenant_id {
        return Err(AppError::TenantMismatch);
    }
    
    Ok(Json(JobResponse {
        job_id: job.id,
        status: job.status.clone(),
        paper_id: job.paper_id,
        chunks_created: job.chunks_processed,
        chunks_total: job.chunks_total,
        progress_percent: job.progress_percent(),
        error_message: job.error_message,
        started_at: job.started_at.map(|dt| dt.to_rfc3339()),
        completed_at: job.completed_at.map(|dt| dt.to_rfc3339()),
        created_at: job.created_at.to_rfc3339(),
    }))
}
