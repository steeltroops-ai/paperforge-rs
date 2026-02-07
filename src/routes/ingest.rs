use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::services::AppState;
use crate::errors::AppError;
use tracing::instrument;

#[derive(Deserialize)]
pub struct IngestRequest {
    pub title: String,
    pub abstract_text: String,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct IngestResponse {
    pub job_id: Uuid,
    pub status: String,
}

#[instrument(skip(state))]
pub async fn ingest_paper(
    State(state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Basic validation
    if payload.title.is_empty() || payload.abstract_text.is_empty() {
        return Err(AppError::ValidationError("Title and abstract are required".to_string()));
    }

    let paper_id = state.ingest_service.ingest_paper(
        payload.title,
        payload.abstract_text,
        payload.source
    ).await?;

    Ok((
        StatusCode::CREATED,
        Json(IngestResponse {
            job_id: paper_id,
            status: "ingested".to_string(),
        }),
    ))
}
