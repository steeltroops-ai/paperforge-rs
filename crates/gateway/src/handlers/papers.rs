//! Paper management handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::AppState;
use paperforge_common::{
    auth::AuthContext,
    db::Repository,
    errors::{AppError, Result},
};

/// Request to create a new paper
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePaperRequest {
    /// Client-provided idempotency key
    #[serde(default)]
    pub idempotency_key: Option<String>,
    
    /// Paper details
    pub paper: PaperInput,
    
    /// Ingestion options
    #[serde(default)]
    pub options: IngestionOptions,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PaperInput {
    #[validate(length(min = 1, max = 1000))]
    pub title: String,
    
    #[validate(length(min = 1, max = 50000))]
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    
    pub source: Option<String>,
    
    pub external_id: Option<String>,
    
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Default, Deserialize)]
pub struct IngestionOptions {
    pub embedding_model: Option<String>,
    pub chunk_strategy: Option<String>,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
}

/// Response after creating a paper
#[derive(Serialize)]
pub struct CreatePaperResponse {
    pub job_id: Uuid,
    pub status: String,
    pub estimated_completion_ms: u64,
    pub poll_url: String,
}

/// Response for getting a paper
#[derive(Serialize)]
pub struct PaperResponse {
    pub id: Uuid,
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub published_at: Option<String>,
    pub metadata: serde_json::Value,
    pub chunk_count: i64,
    pub created_at: String,
}

/// Create a new paper and start async ingestion
pub async fn create_paper(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<CreatePaperRequest>,
) -> Result<(StatusCode, Json<CreatePaperResponse>)> {
    // Validate request
    request.paper.validate().map_err(|e| AppError::Validation {
        message: e.to_string(),
        field: None,
    })?;
    
    let repo = Repository::new(state.db.clone());
    
    // Check for duplicate via idempotency key
    if let Some(ref key) = request.idempotency_key {
        if let Some(existing_job) = repo.find_job_by_idempotency_key(auth.tenant_id, key).await? {
            // Return existing job (idempotent response)
            return Ok((StatusCode::OK, Json(CreatePaperResponse {
                job_id: existing_job.id,
                status: existing_job.status.clone(),
                estimated_completion_ms: 0,
                poll_url: format!("/v2/jobs/{}", existing_job.id),
            })));
        }
    }
    
    // Create the ingestion job
    let job = repo.create_job(auth.tenant_id, request.idempotency_key.clone()).await?;
    
    // TODO: Send to ingestion queue for async processing
    // For now, we'll process synchronously (Phase 1 limitation)
    
    tracing::info!(
        job_id = %job.id,
        tenant_id = %auth.tenant_id,
        title = %request.paper.title,
        "Paper ingestion job created"
    );
    
    Ok((StatusCode::ACCEPTED, Json(CreatePaperResponse {
        job_id: job.id,
        status: "pending".to_string(),
        estimated_completion_ms: 5000,
        poll_url: format!("/v2/jobs/{}", job.id),
    })))
}

/// Get a paper by ID
pub async fn get_paper(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(paper_id): Path<Uuid>,
) -> Result<Json<PaperResponse>> {
    let repo = Repository::new(state.db.clone());
    
    let paper = repo.find_paper_by_id(paper_id)
        .await?
        .ok_or_else(|| AppError::PaperNotFound { 
            id: paper_id.to_string() 
        })?;
    
    // Verify tenant access
    if paper.tenant_id != auth.tenant_id {
        return Err(AppError::TenantMismatch);
    }
    
    // Get chunk count
    let chunks = repo.get_chunks_by_paper(paper_id).await?;
    
    Ok(Json(PaperResponse {
        id: paper.id,
        title: paper.title,
        abstract_text: paper.abstract_text,
        source: paper.source,
        external_id: paper.external_id,
        published_at: paper.published_at.map(|dt| dt.to_rfc3339()),
        metadata: paper.metadata,
        chunk_count: chunks.len() as i64,
        created_at: paper.created_at.to_rfc3339(),
    }))
}

/// Delete a paper
pub async fn delete_paper(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(paper_id): Path<Uuid>,
) -> Result<StatusCode> {
    let repo = Repository::new(state.db.clone());
    
    // Verify paper exists and belongs to tenant
    let paper = repo.find_paper_by_id(paper_id)
        .await?
        .ok_or_else(|| AppError::PaperNotFound { 
            id: paper_id.to_string() 
        })?;
    
    if paper.tenant_id != auth.tenant_id {
        return Err(AppError::TenantMismatch);
    }
    
    repo.delete_paper(paper_id).await?;
    
    tracing::info!(
        paper_id = %paper_id,
        tenant_id = %auth.tenant_id,
        "Paper deleted"
    );
    
    Ok(StatusCode::NO_CONTENT)
}
