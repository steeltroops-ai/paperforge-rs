//! Session management handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use paperforge_common::{
    auth::AuthContext,
    db::Repository,
    errors::{AppError, Result},
};

/// Create session request
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Create session response
#[derive(Serialize)]
pub struct CreateSessionResponse {
    pub session_id: Uuid,
    pub expires_at: String,
}

/// Session state response
#[derive(Serialize)]
pub struct SessionResponse {
    pub session_id: Uuid,
    pub state: serde_json::Value,
    pub created_at: String,
    pub last_active_at: String,
    pub expires_at: String,
}

/// Track event request
#[derive(Debug, Deserialize)]
pub struct TrackEventRequest {
    pub event: String,
    pub data: serde_json::Value,
}

/// Create a new session
pub async fn create_session(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<CreateSessionResponse>)> {
    let repo = Repository::new(state.db.clone());
    let session_id = Uuid::new_v4();
    
    let initial_state = serde_json::json!({
        "queries": [],
        "viewed_papers": [],
        "clicked_results": [],
        "preferred_topics": {},
        "metadata": request.metadata,
    });
    
    let session = repo.upsert_session(
        auth.tenant_id,
        session_id,
        initial_state,
        30, // 30 minute TTL
    ).await?;
    
    tracing::info!(
        session_id = %session_id,
        tenant_id = %auth.tenant_id,
        "Session created"
    );
    
    Ok((StatusCode::CREATED, Json(CreateSessionResponse {
        session_id: session.id,
        expires_at: session.expires_at.to_rfc3339(),
    })))
}

/// Get session state
pub async fn get_session(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionResponse>> {
    let repo = Repository::new(state.db.clone());
    
    let session = repo.find_session(session_id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound { 
            id: session_id.to_string() 
        })?;
    
    // Verify tenant access
    if session.tenant_id != auth.tenant_id {
        return Err(AppError::TenantMismatch);
    }
    
    // Check expiration
    if session.is_expired() {
        return Err(AppError::SessionNotFound { 
            id: session_id.to_string() 
        });
    }
    
    Ok(Json(SessionResponse {
        session_id: session.id,
        state: session.state,
        created_at: session.created_at.to_rfc3339(),
        last_active_at: session.last_active_at.to_rfc3339(),
        expires_at: session.expires_at.to_rfc3339(),
    }))
}

/// Track user event in session
pub async fn track_event(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(session_id): Path<Uuid>,
    Json(request): Json<TrackEventRequest>,
) -> Result<StatusCode> {
    let repo = Repository::new(state.db.clone());
    
    let session = repo.find_session(session_id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound { 
            id: session_id.to_string() 
        })?;
    
    // Verify tenant access
    if session.tenant_id != auth.tenant_id {
        return Err(AppError::TenantMismatch);
    }
    
    // Update session state with event
    let mut state = session.state.clone();
    
    match request.event.as_str() {
        "click" => {
            if let Some(clicked) = state.get_mut("clicked_results") {
                if let Some(arr) = clicked.as_array_mut() {
                    arr.push(request.data.clone());
                }
            }
        }
        "view_paper" => {
            if let Some(viewed) = state.get_mut("viewed_papers") {
                if let Some(arr) = viewed.as_array_mut() {
                    arr.push(request.data.clone());
                }
            }
        }
        "query" => {
            if let Some(queries) = state.get_mut("queries") {
                if let Some(arr) = queries.as_array_mut() {
                    arr.push(serde_json::json!({
                        "query": request.data.get("query"),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }));
                }
            }
        }
        _ => {
            tracing::debug!(event = %request.event, "Unknown event type");
        }
    }
    
    // Update session
    repo.upsert_session(auth.tenant_id, session_id, state, 30).await?;
    
    tracing::debug!(
        session_id = %session_id,
        event = %request.event,
        "Event tracked"
    );
    
    Ok(StatusCode::NO_CONTENT)
}
