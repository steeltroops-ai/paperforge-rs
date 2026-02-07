//! Citation graph handlers

use axum::{
    extract::{Path, State},
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

/// Citation graph response
#[derive(Serialize)]
pub struct CitationResponse {
    pub paper_id: Uuid,
    pub paper_title: String,
    pub citations: CitationGraph,
    pub stats: CitationStats,
}

#[derive(Serialize)]
pub struct CitationGraph {
    pub outgoing: Vec<CitationLink>,
    pub incoming: Vec<CitationLink>,
}

#[derive(Serialize)]
pub struct CitationLink {
    pub paper_id: Uuid,
    pub paper_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Serialize)]
pub struct CitationStats {
    pub outgoing_count: usize,
    pub incoming_count: usize,
}

/// Traverse citations request
#[derive(Debug, Deserialize)]
pub struct TraverseCitationsRequest {
    pub seed_papers: Vec<Uuid>,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_hops")]
    pub max_hops: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_direction() -> String { "both".to_string() }
fn default_hops() -> usize { 2 }
fn default_limit() -> usize { 50 }

/// Traverse citations response
#[derive(Serialize)]
pub struct TraverseCitationsResponse {
    pub seed_papers: Vec<Uuid>,
    pub papers: Vec<TraversedPaper>,
    pub graph: GraphData,
}

#[derive(Serialize)]
pub struct TraversedPaper {
    pub paper_id: Uuid,
    pub title: String,
    pub hop_distance: usize,
    pub propagation_score: f64,
}

#[derive(Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Serialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub title: String,
    pub hop: usize,
}

#[derive(Serialize)]
pub struct GraphEdge {
    pub source: Uuid,
    pub target: Uuid,
}

/// Get citations for a paper
pub async fn get_citations(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(paper_id): Path<Uuid>,
) -> Result<Json<CitationResponse>> {
    let repo = Repository::new(state.db.clone());
    
    // Get paper details
    let paper = repo.find_paper_by_id(paper_id)
        .await?
        .ok_or_else(|| AppError::PaperNotFound { 
            id: paper_id.to_string() 
        })?;
    
    // Verify tenant access
    if paper.tenant_id != auth.tenant_id {
        return Err(AppError::TenantMismatch);
    }
    
    // Get citations
    let (outgoing, incoming) = repo.get_citations(paper_id).await?;
    
    // Convert to response format (would need to join with papers table for titles)
    let outgoing_links: Vec<CitationLink> = outgoing.iter().map(|c| {
        CitationLink {
            paper_id: c.cited_paper_id,
            paper_title: "Unknown".to_string(), // TODO: Join with papers
            context: c.citation_context.clone(),
        }
    }).collect();
    
    let incoming_links: Vec<CitationLink> = incoming.iter().map(|c| {
        CitationLink {
            paper_id: c.citing_paper_id,
            paper_title: "Unknown".to_string(), // TODO: Join with papers
            context: c.citation_context.clone(),
        }
    }).collect();
    
    Ok(Json(CitationResponse {
        paper_id: paper.id,
        paper_title: paper.title,
        citations: CitationGraph {
            outgoing: outgoing_links,
            incoming: incoming_links,
        },
        stats: CitationStats {
            outgoing_count: outgoing.len(),
            incoming_count: incoming.len(),
        },
    }))
}

/// Traverse citation graph from seed papers
pub async fn traverse_citations(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<TraverseCitationsRequest>,
) -> Result<Json<TraverseCitationsResponse>> {
    let repo = Repository::new(state.db.clone());
    
    if request.seed_papers.is_empty() {
        return Err(AppError::Validation {
            message: "At least one seed paper required".to_string(),
            field: Some("seed_papers".to_string()),
        });
    }
    
    if request.seed_papers.len() > 10 {
        return Err(AppError::Validation {
            message: "Maximum 10 seed papers".to_string(),
            field: Some("seed_papers".to_string()),
        });
    }
    
    // Verify all seed papers exist and belong to tenant
    for &paper_id in &request.seed_papers {
        let paper = repo.find_paper_by_id(paper_id)
            .await?
            .ok_or_else(|| AppError::PaperNotFound { 
                id: paper_id.to_string() 
            })?;
        
        if paper.tenant_id != auth.tenant_id {
            return Err(AppError::TenantMismatch);
        }
    }
    
    // TODO: Implement actual BFS/DFS traversal with citation propagation scoring
    // For now, return placeholder response
    
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut papers = Vec::new();
    
    // Add seed papers as hop 0
    for &seed_id in &request.seed_papers {
        if let Some(paper) = repo.find_paper_by_id(seed_id).await? {
            nodes.push(GraphNode {
                id: seed_id,
                title: paper.title.clone(),
                hop: 0,
            });
            papers.push(TraversedPaper {
                paper_id: seed_id,
                title: paper.title,
                hop_distance: 0,
                propagation_score: 1.0,
            });
        }
        
        // Get first-hop citations
        let (outgoing, incoming) = repo.get_citations(seed_id).await?;
        
        for citation in outgoing.iter().take(5) {
            if let Some(cited_paper) = repo.find_paper_by_id(citation.cited_paper_id).await? {
                if cited_paper.tenant_id == auth.tenant_id {
                    nodes.push(GraphNode {
                        id: cited_paper.id,
                        title: cited_paper.title.clone(),
                        hop: 1,
                    });
                    edges.push(GraphEdge {
                        source: seed_id,
                        target: cited_paper.id,
                    });
                    papers.push(TraversedPaper {
                        paper_id: cited_paper.id,
                        title: cited_paper.title,
                        hop_distance: 1,
                        propagation_score: 0.8,
                    });
                }
            }
        }
        
        if request.direction == "both" || request.direction == "incoming" {
            for citation in incoming.iter().take(5) {
                if let Some(citing_paper) = repo.find_paper_by_id(citation.citing_paper_id).await? {
                    if citing_paper.tenant_id == auth.tenant_id {
                        nodes.push(GraphNode {
                            id: citing_paper.id,
                            title: citing_paper.title.clone(),
                            hop: 1,
                        });
                        edges.push(GraphEdge {
                            source: citing_paper.id,
                            target: seed_id,
                        });
                        papers.push(TraversedPaper {
                            paper_id: citing_paper.id,
                            title: citing_paper.title,
                            hop_distance: 1,
                            propagation_score: 0.7,
                        });
                    }
                }
            }
        }
    }
    
    // Truncate to limit
    papers.truncate(request.limit);
    
    Ok(Json(TraverseCitationsResponse {
        seed_papers: request.seed_papers,
        papers,
        graph: GraphData { nodes, edges },
    }))
}
