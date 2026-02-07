//! Multi-hop Reasoner - Performs iterative query refinement
//!
//! Provides:
//! - Chain-of-thought reasoning
//! - Multi-hop query execution
//! - Fact extraction
//! - Confidence scoring

use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Reasoning chain result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChain {
    /// Original query
    pub original_query: String,
    
    /// Reasoning hops
    pub hops: Vec<ReasoningHop>,
    
    /// Extracted facts across all hops
    pub all_facts: Vec<String>,
    
    /// Overall confidence
    pub confidence: f32,
    
    /// Number of hops executed
    pub hop_count: usize,
}

/// Single reasoning hop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningHop {
    /// Query at this hop
    pub query: String,
    
    /// Hop number (1-indexed)
    pub hop_number: usize,
    
    /// Facts extracted from retrieved documents
    pub facts: Vec<String>,
    
    /// Number of documents considered
    pub docs_considered: usize,
    
    /// Next query (if more hops needed)
    pub next_query: Option<String>,
    
    /// Rationale for next query
    pub rationale: Option<String>,
    
    /// Confidence in this hop
    pub confidence: f32,
}

/// Reasoner configuration
#[derive(Debug, Clone)]
pub struct ReasonerConfig {
    /// Maximum hops to perform
    pub max_hops: usize,
    
    /// Minimum confidence to continue
    pub min_confidence: f32,
    
    /// Maximum facts per hop
    pub max_facts_per_hop: usize,
    
    /// Enable LLM-based fact extraction
    pub use_llm: bool,
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        Self {
            max_hops: 3,
            min_confidence: 0.5,
            max_facts_per_hop: 5,
            use_llm: true,
        }
    }
}

/// Context from search results
#[derive(Debug, Clone)]
pub struct ReasonerContext {
    pub content: String,
    pub source: String,
    pub score: f32,
}

/// Reasoner for multi-hop reasoning
pub struct Reasoner {
    config: ReasonerConfig,
}

impl Reasoner {
    /// Create a new reasoner
    pub fn new(config: ReasonerConfig) -> Self {
        Self { config }
    }
    
    /// Perform multi-hop reasoning
    pub async fn reason<F, Fut>(
        &self,
        initial_query: &str,
        search_fn: F,
    ) -> Result<ReasoningChain>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<ReasonerContext>>>,
    {
        let mut hops = Vec::new();
        let mut all_facts = Vec::new();
        let mut current_query = initial_query.to_string();
        let mut seen_facts: HashSet<String> = HashSet::new();
        
        for hop_num in 1..=self.config.max_hops {
            // Execute search for current query
            let contexts = search_fn(current_query.clone()).await?;
            
            if contexts.is_empty() {
                break;
            }
            
            // Extract facts from contexts
            let hop_facts = self.extract_facts(&contexts, &current_query);
            
            // Deduplicate facts
            let new_facts: Vec<String> = hop_facts
                .into_iter()
                .filter(|f| seen_facts.insert(f.clone()))
                .take(self.config.max_facts_per_hop)
                .collect();
            
            // Generate next query based on gaps
            let (next_query, rationale) = if hop_num < self.config.max_hops {
                self.generate_next_query(&current_query, &new_facts)
            } else {
                (None, None)
            };
            
            // Calculate hop confidence
            let confidence = self.calculate_hop_confidence(&contexts, &new_facts);
            
            let hop = ReasoningHop {
                query: current_query.clone(),
                hop_number: hop_num,
                facts: new_facts.clone(),
                docs_considered: contexts.len(),
                next_query: next_query.clone(),
                rationale,
                confidence,
            };
            
            all_facts.extend(new_facts);
            hops.push(hop);
            
            // Check if we should continue
            if confidence < self.config.min_confidence {
                break;
            }
            
            match next_query {
                Some(q) => current_query = q,
                None => break,
            }
        }
        
        // Calculate overall confidence
        let overall_confidence = if hops.is_empty() {
            0.0
        } else {
            hops.iter().map(|h| h.confidence).sum::<f32>() / hops.len() as f32
        };
        
        Ok(ReasoningChain {
            original_query: initial_query.to_string(),
            hops: hops.clone(),
            all_facts,
            confidence: overall_confidence,
            hop_count: hops.len(),
        })
    }
    
    /// Extract facts from contexts (pattern-based)
    fn extract_facts(&self, contexts: &[ReasonerContext], query: &str) -> Vec<String> {
        let mut facts = Vec::new();
        let query_words: HashSet<_> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|s| s.to_string())
            .collect();
        
        for ctx in contexts {
            // Split into sentences
            let sentences = self.split_sentences(&ctx.content);
            
            for sentence in sentences {
                // Check if sentence is relevant to query
                let sentence_lower = sentence.to_lowercase();
                let relevance = query_words.iter()
                    .filter(|w| sentence_lower.contains(w.as_str()))
                    .count();
                
                // Include if at least 2 query words present
                if relevance >= 2 && sentence.len() > 20 && sentence.len() < 500 {
                    let fact = sentence.trim().to_string();
                    if !facts.contains(&fact) {
                        facts.push(fact);
                    }
                }
            }
        }
        
        // Sort by length (prefer concise facts)
        facts.sort_by_key(|f| f.len());
        facts.truncate(self.config.max_facts_per_hop * 2);
        
        facts
    }
    
    /// Generate next query based on gaps in knowledge
    fn generate_next_query(
        &self,
        current_query: &str,
        facts: &[String],
    ) -> (Option<String>, Option<String>) {
        if facts.is_empty() {
            // No facts found, try rephrasing
            if current_query.contains("how") {
                let rephrased = current_query.replace("how", "methods for");
                return (Some(rephrased.clone()), Some("Rephrasing to find methods".to_string()));
            }
            return (None, None);
        }
        
        // Extract potential follow-up concepts from facts
        let all_facts_text = facts.join(" ");
        let potential_concepts = self.extract_potential_concepts(&all_facts_text, current_query);
        
        if let Some(concept) = potential_concepts.first() {
            let next = format!("{} {}", current_query, concept);
            let rationale = format!("Exploring related concept: {}", concept);
            return (Some(next), Some(rationale));
        }
        
        (None, None)
    }
    
    /// Extract potential follow-up concepts
    fn extract_potential_concepts(&self, text: &str, original_query: &str) -> Vec<String> {
        let original_words: HashSet<_> = original_query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        
        let mut concepts: Vec<String> = Vec::new();
        
        // Look for capitalized terms not in original query
        for word in text.split_whitespace() {
            let clean: String = word.chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            
            if clean.len() > 4 
                && !original_words.contains(&clean.to_lowercase())
                && clean.chars().next().map_or(false, |c| c.is_uppercase())
            {
                if !concepts.contains(&clean) {
                    concepts.push(clean);
                }
            }
        }
        
        concepts.truncate(3);
        concepts
    }
    
    /// Calculate confidence for a hop
    fn calculate_hop_confidence(&self, contexts: &[ReasonerContext], facts: &[String]) -> f32 {
        if contexts.is_empty() || facts.is_empty() {
            return 0.3;
        }
        
        // Average score of contexts
        let avg_score = contexts.iter().map(|c| c.score).sum::<f32>() / contexts.len() as f32;
        
        // Fact coverage
        let fact_score = (facts.len() as f32 / self.config.max_facts_per_hop as f32).min(1.0);
        
        (avg_score + fact_score) / 2.0
    }
    
    /// Split text into sentences
    fn split_sentences(&self, text: &str) -> Vec<String> {
        let delimiters = ['.', '?', '!'];
        let mut sentences = Vec::new();
        let mut current = String::new();
        
        for ch in text.chars() {
            current.push(ch);
            if delimiters.contains(&ch) {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                current.clear();
            }
        }
        
        // Add remaining text
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            sentences.push(trimmed);
        }
        
        sentences
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reasoning() {
        let reasoner = Reasoner::new(ReasonerConfig::default());
        
        let mock_search = |_query: String| async {
            Ok(vec![
                ReasonerContext {
                    content: "Transformers use attention mechanisms. The attention mechanism allows models to focus on relevant parts.".to_string(),
                    source: "paper1".to_string(),
                    score: 0.8,
                },
            ])
        };
        
        let chain = reasoner.reason("What is attention in transformers?", mock_search).await.unwrap();
        
        assert!(!chain.hops.is_empty());
        assert!(chain.confidence > 0.0);
    }
    
    #[test]
    fn test_sentence_splitting() {
        let reasoner = Reasoner::new(ReasonerConfig::default());
        
        let text = "First sentence. Second sentence! Third sentence?";
        let sentences = reasoner.split_sentences(text);
        
        assert_eq!(sentences.len(), 3);
    }
}
