//! Paper ingestion route handler
//!
//! Handles POST /ingest requests with:
//! - Input validation (title, abstract, source)
//! - Idempotency key generation and deduplication
//! - Paper creation and chunking

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;
use crate::services::AppState;
use crate::errors::AppError;
use crate::middleware::generate_idempotency_hash;
use tracing::instrument;

/// Maximum title length in characters
const MAX_TITLE_LENGTH: usize = 1000;
/// Maximum abstract length in characters  
const MAX_ABSTRACT_LENGTH: usize = 100_000;
/// Minimum abstract length for meaningful content
const MIN_ABSTRACT_LENGTH: usize = 50;

#[derive(Debug, Deserialize, Validate)]
pub struct IngestRequest {
    #[validate(length(min = 1, max = 1000, message = "Title must be 1-1000 characters"))]
    pub title: String,
    
    #[validate(length(min = 50, max = 100000, message = "Abstract must be 50-100000 characters"))]
    pub abstract_text: String,
    
    #[validate(length(max = 100, message = "Source must be max 100 characters"))]
    pub source: Option<String>,
    
    /// Optional client-provided idempotency key
    /// If not provided, one will be generated from title+abstract hash
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub paper_id: Uuid,
    pub status: String,
    pub idempotency_key: String,
    pub chunks_created: usize,
    /// Indicates if this was a duplicate (previously ingested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<bool>,
}

/// Paper ingestion endpoint
/// 
/// # Validation
/// - Title: 1-1000 characters, required
/// - Abstract: 50-100000 characters, required
/// - Source: optional, max 100 characters
/// 
/// # Idempotency
/// - Uses idempotency_key if provided
/// - Otherwise generates from SHA256(title|abstract)
/// - Returns existing paper if duplicate detected (status 200)
/// - Returns new paper if created (status 201)
#[instrument(
    skip(state, payload),
    fields(
        title_len = payload.title.len(),
        abstract_len = payload.abstract_text.len(),
    )
)]
pub async fn ingest_paper(
    State(state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Validate input using validator crate
    payload.validate().map_err(|e| {
        let messages: Vec<String> = e.field_errors()
            .into_iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| {
                    format!("{}: {}", field, e.message.as_ref().map(|m| m.to_string()).unwrap_or_default())
                })
            })
            .collect();
        AppError::ValidationError(messages.join("; "))
    })?;
    
    // 2. Additional validation for whitespace-only content
    if payload.title.trim().is_empty() {
        return Err(AppError::MissingField("title".to_string()));
    }
    if payload.abstract_text.trim().is_empty() {
        return Err(AppError::MissingField("abstract_text".to_string()));
    }
    
    // 3. Generate or use idempotency key
    let idempotency_key = payload.idempotency_key.clone().unwrap_or_else(|| {
        generate_idempotency_hash(&payload.title, &payload.abstract_text)
    });
    
    tracing::debug!(idempotency_key = %idempotency_key, "Processing ingestion request");
    
    // 4. Check for existing paper with same idempotency key
    if let Some(existing_paper) = state.repo.find_by_idempotency_key(&idempotency_key).await? {
        tracing::info!(
            paper_id = %existing_paper.id,
            idempotency_key = %idempotency_key,
            "Duplicate paper detected, returning existing"
        );
        
        // Return existing paper with 200 OK (not 201 Created)
        return Ok((
            StatusCode::OK,
            Json(IngestResponse {
                paper_id: existing_paper.id,
                status: "exists".to_string(),
                idempotency_key,
                chunks_created: 0,
                duplicate: Some(true),
            }),
        ));
    }
    
    // 5. Ingest new paper
    let (paper_id, chunks_created) = state.ingest_service.ingest_paper(
        payload.title,
        payload.abstract_text,
        payload.source,
        idempotency_key.clone(),
    ).await?;

    tracing::info!(
        paper_id = %paper_id,
        chunks_created = chunks_created,
        "Paper ingested successfully"
    );

    Ok((
        StatusCode::CREATED,
        Json(IngestResponse {
            paper_id,
            status: "ingested".to_string(),
            idempotency_key,
            chunks_created,
            duplicate: None,
        }),
    ))
}
