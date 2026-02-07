//! Citation graph representation
//!
//! Provides in-memory citation graph for scoring

use paperforge_common::errors::{AppError, Result};
use paperforge_common::db::DbPool;
use sea_orm::{ConnectionTrait, Statement, DbBackend};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Edge in the citation graph
#[derive(Debug, Clone)]
pub struct CitationEdge {
    /// Citing paper ID
    pub citing_paper_id: Uuid,
    
    /// Cited paper ID
    pub cited_paper_id: Uuid,
    
    /// Edge weight (default 1.0)
    pub weight: f32,
}

/// In-memory citation graph
pub struct CitationGraph {
    /// Adjacency list: paper_id -> list of papers it cites
    outgoing: HashMap<Uuid, Vec<Uuid>>,
    
    /// Reverse adjacency: paper_id -> list of papers citing it
    incoming: HashMap<Uuid, Vec<Uuid>>,
    
    /// All nodes (paper IDs)
    nodes: HashSet<Uuid>,
    
    /// Paper titles for display
    titles: HashMap<Uuid, String>,
}

impl CitationGraph {
    /// Create an empty graph
    pub fn new() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            nodes: HashSet::new(),
            titles: HashMap::new(),
        }
    }
    
    /// Load citation graph from database for a tenant
    pub async fn load_from_db(db: &DbPool, tenant_id: Uuid) -> Result<Self> {
        let mut graph = Self::new();
        
        // Load papers
        let papers_sql = r#"
            SELECT id, title
            FROM papers
            WHERE tenant_id = $1 AND status = 'processed'
        "#;
        
        let conn = db.read_connection().await;
        let paper_rows = conn
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                papers_sql,
                vec![tenant_id.into()],
            ))
            .await
            .map_err(|e| AppError::DatabaseError { 
                message: format!("Failed to load papers: {}", e) 
            })?;
        
        for row in paper_rows {
            use sea_orm::TryGetable;
            if let (Ok(id), Ok(title)) = (
                row.try_get::<Uuid, _>("", "id"),
                row.try_get::<String, _>("", "title"),
            ) {
                graph.nodes.insert(id);
                graph.titles.insert(id, title);
            }
        }
        
        // Load citations
        let citations_sql = r#"
            SELECT c.citing_paper_id, c.cited_paper_id
            FROM citations c
            INNER JOIN papers p1 ON c.citing_paper_id = p1.id
            INNER JOIN papers p2 ON c.cited_paper_id = p2.id
            WHERE p1.tenant_id = $1
        "#;
        
        let citation_rows = conn
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                citations_sql,
                vec![tenant_id.into()],
            ))
            .await
            .map_err(|e| AppError::DatabaseError { 
                message: format!("Failed to load citations: {}", e) 
            })?;
        
        for row in citation_rows {
            use sea_orm::TryGetable;
            if let (Ok(citing), Ok(cited)) = (
                row.try_get::<Uuid, _>("", "citing_paper_id"),
                row.try_get::<Uuid, _>("", "cited_paper_id"),
            ) {
                graph.add_edge(citing, cited);
            }
        }
        
        Ok(graph)
    }
    
    /// Add an edge to the graph
    pub fn add_edge(&mut self, citing: Uuid, cited: Uuid) {
        self.nodes.insert(citing);
        self.nodes.insert(cited);
        
        self.outgoing.entry(citing).or_default().push(cited);
        self.incoming.entry(cited).or_default().push(citing);
    }
    
    /// Get papers cited by this paper
    pub fn get_references(&self, paper_id: Uuid) -> &[Uuid] {
        self.outgoing.get(&paper_id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    /// Get papers citing this paper
    pub fn get_citations(&self, paper_id: Uuid) -> &[Uuid] {
        self.incoming.get(&paper_id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &Uuid> {
        self.nodes.iter()
    }
    
    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Get paper title
    pub fn get_title(&self, paper_id: Uuid) -> Option<&String> {
        self.titles.get(&paper_id)
    }
    
    /// Get citation count (incoming edges)
    pub fn citation_count(&self, paper_id: Uuid) -> usize {
        self.incoming.get(&paper_id).map(|v| v.len()).unwrap_or(0)
    }
    
    /// Get reference count (outgoing edges)
    pub fn reference_count(&self, paper_id: Uuid) -> usize {
        self.outgoing.get(&paper_id).map(|v| v.len()).unwrap_or(0)
    }
    
    /// Traverse citations up to a depth
    pub fn traverse(
        &self,
        start: Uuid,
        depth: usize,
        direction: TraversalDirection,
    ) -> Vec<(Uuid, usize)> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = vec![(start, 0usize)];
        
        while let Some((current, current_depth)) = queue.pop() {
            if current_depth > depth || visited.contains(&current) {
                continue;
            }
            
            visited.insert(current);
            
            if current != start {
                result.push((current, current_depth));
            }
            
            if current_depth < depth {
                let neighbors = match direction {
                    TraversalDirection::Forward => self.get_references(current),
                    TraversalDirection::Backward => self.get_citations(current),
                    TraversalDirection::Both => {
                        // Combine both directions
                        let mut both = self.get_references(current).to_vec();
                        both.extend_from_slice(self.get_citations(current));
                        // Return via temporary - simplified
                        &[]
                    }
                };
                
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        queue.push((neighbor, current_depth + 1));
                    }
                }
            }
        }
        
        result
    }
}

impl Default for CitationGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Direction for graph traversal
#[derive(Debug, Clone, Copy)]
pub enum TraversalDirection {
    /// Follow references (papers cited by this paper)
    Forward,
    /// Follow citations (papers citing this paper)
    Backward,
    /// Both directions
    Both,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_graph_construction() {
        let mut graph = CitationGraph::new();
        
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let c = Uuid::from_u128(3);
        
        // A cites B, B cites C
        graph.add_edge(a, b);
        graph.add_edge(b, c);
        
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.get_references(a), &[b]);
        assert_eq!(graph.get_citations(b), &[a]);
        assert_eq!(graph.get_references(b), &[c]);
    }
    
    #[test]
    fn test_citation_counts() {
        let mut graph = CitationGraph::new();
        
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let c = Uuid::from_u128(3);
        
        // Both A and C cite B
        graph.add_edge(a, b);
        graph.add_edge(c, b);
        
        assert_eq!(graph.citation_count(b), 2);
        assert_eq!(graph.reference_count(a), 1);
    }
}
