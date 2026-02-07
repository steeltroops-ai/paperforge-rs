//! Query Parser - Understands user intent and extracts entities
//!
//! Provides:
//! - Intent classification
//! - Entity extraction (concepts, authors, methods)
//! - Query expansion with synonyms

use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query understanding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryUnderstanding {
    /// Original query text
    pub original_query: String,
    
    /// Detected intent
    pub intent: QueryIntent,
    
    /// Extracted entities
    pub entities: Vec<Entity>,
    
    /// Expanded query terms (synonyms, related concepts)
    pub expanded_terms: Vec<String>,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Query intent classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryIntent {
    /// Looking for specific information
    Factual,
    /// Comparing concepts or methods
    Comparison,
    /// Seeking understanding/explanation
    Exploratory,
    /// Looking for methodology/how-to
    Procedural,
    /// Reviewing state of the art
    Survey,
    /// Unknown/general
    General,
}

/// Extracted entity from query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity text
    pub text: String,
    
    /// Entity type
    pub entity_type: EntityType,
    
    /// Confidence score
    pub confidence: f32,
    
    /// Position in original query (start, end)
    pub span: Option<(usize, usize)>,
}

/// Types of entities we can extract
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    /// Scientific concept
    Concept,
    /// Author name
    Author,
    /// Research method/algorithm
    Method,
    /// Dataset name
    Dataset,
    /// Venue (conference/journal)
    Venue,
    /// Time reference
    Temporal,
    /// General term
    Term,
}

/// Query parser configuration
#[derive(Debug, Clone)]
pub struct QueryParserConfig {
    /// Enable synonym expansion
    pub enable_expansion: bool,
    
    /// Maximum expanded terms
    pub max_expansions: usize,
    
    /// Minimum entity confidence
    pub min_entity_confidence: f32,
    
    /// Use LLM for complex queries
    pub use_llm_fallback: bool,
}

impl Default for QueryParserConfig {
    fn default() -> Self {
        Self {
            enable_expansion: true,
            max_expansions: 5,
            min_entity_confidence: 0.6,
            use_llm_fallback: true,
        }
    }
}

/// Query parser for understanding user queries
pub struct QueryParser {
    config: QueryParserConfig,
    
    /// Synonym dictionary for expansion
    synonyms: HashMap<String, Vec<String>>,
    
    /// Stop words to filter
    stop_words: Vec<String>,
}

impl QueryParser {
    /// Create a new query parser
    pub fn new(config: QueryParserConfig) -> Self {
        let synonyms = Self::load_default_synonyms();
        let stop_words = Self::load_stop_words();
        
        Self {
            config,
            synonyms,
            stop_words,
        }
    }
    
    /// Parse a query and extract understanding
    pub async fn parse(&self, query: &str) -> Result<QueryUnderstanding> {
        let query = query.trim().to_lowercase();
        
        // Detect intent
        let intent = self.detect_intent(&query);
        
        // Extract entities
        let entities = self.extract_entities(&query);
        
        // Expand query terms
        let expanded_terms = if self.config.enable_expansion {
            self.expand_query(&query)
        } else {
            vec![]
        };
        
        // Calculate confidence based on extraction quality
        let confidence = self.calculate_confidence(&intent, &entities);
        
        Ok(QueryUnderstanding {
            original_query: query,
            intent,
            entities,
            expanded_terms,
            confidence,
        })
    }
    
    /// Detect query intent using heuristics
    fn detect_intent(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();
        
        // Check for comparison patterns
        if query_lower.contains(" vs ") 
            || query_lower.contains(" versus ")
            || query_lower.contains("compare")
            || query_lower.contains("difference between")
        {
            return QueryIntent::Comparison;
        }
        
        // Check for procedural patterns
        if query_lower.starts_with("how to")
            || query_lower.starts_with("how do")
            || query_lower.contains("step by step")
            || query_lower.contains("implement")
        {
            return QueryIntent::Procedural;
        }
        
        // Check for survey patterns
        if query_lower.contains("state of the art")
            || query_lower.contains("survey")
            || query_lower.contains("review of")
            || query_lower.contains("overview")
        {
            return QueryIntent::Survey;
        }
        
        // Check for factual patterns
        if query_lower.starts_with("what is")
            || query_lower.starts_with("who is")
            || query_lower.starts_with("when")
            || query_lower.starts_with("define")
        {
            return QueryIntent::Factual;
        }
        
        // Check for exploratory patterns
        if query_lower.starts_with("why")
            || query_lower.starts_with("explain")
            || query_lower.contains("understand")
        {
            return QueryIntent::Exploratory;
        }
        
        QueryIntent::General
    }
    
    /// Extract entities from query (basic pattern matching)
    fn extract_entities(&self, query: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        let words: Vec<&str> = query.split_whitespace().collect();
        
        // Pattern: capitalized sequences (potential concepts/methods)
        let mut i = 0;
        while i < words.len() {
            let word = words[i];
            
            // Skip stop words
            if self.is_stop_word(word) {
                i += 1;
                continue;
            }
            
            // Check for known method patterns
            if self.is_method_keyword(word) {
                entities.push(Entity {
                    text: word.to_string(),
                    entity_type: EntityType::Method,
                    confidence: 0.7,
                    span: None,
                });
            }
            
            // Check for temporal references
            if self.is_temporal(word) {
                entities.push(Entity {
                    text: word.to_string(),
                    entity_type: EntityType::Temporal,
                    confidence: 0.9,
                    span: None,
                });
            }
            
            // Multi-word concept detection (e.g., "machine learning")
            if i + 1 < words.len() {
                let bigram = format!("{} {}", word, words[i + 1]);
                if self.is_known_concept(&bigram) {
                    entities.push(Entity {
                        text: bigram,
                        entity_type: EntityType::Concept,
                        confidence: 0.85,
                        span: None,
                    });
                    i += 2;
                    continue;
                }
            }
            
            // Single word concepts (if not stop word and looks like a term)
            if word.len() > 3 && !self.is_stop_word(word) {
                entities.push(Entity {
                    text: word.to_string(),
                    entity_type: EntityType::Term,
                    confidence: 0.5,
                    span: None,
                });
            }
            
            i += 1;
        }
        
        // Filter by confidence
        entities
            .into_iter()
            .filter(|e| e.confidence >= self.config.min_entity_confidence)
            .collect()
    }
    
    /// Expand query with synonyms
    fn expand_query(&self, query: &str) -> Vec<String> {
        let mut expansions = Vec::new();
        
        for word in query.split_whitespace() {
            if let Some(syns) = self.synonyms.get(&word.to_lowercase()) {
                for syn in syns.iter().take(self.config.max_expansions) {
                    if !expansions.contains(syn) {
                        expansions.push(syn.clone());
                    }
                }
            }
        }
        
        expansions.truncate(self.config.max_expansions);
        expansions
    }
    
    /// Calculate overall confidence
    fn calculate_confidence(&self, intent: &QueryIntent, entities: &[Entity]) -> f32 {
        let intent_conf = match intent {
            QueryIntent::General => 0.5,
            _ => 0.8,
        };
        
        let entity_conf = if entities.is_empty() {
            0.4
        } else {
            entities.iter().map(|e| e.confidence).sum::<f32>() / entities.len() as f32
        };
        
        (intent_conf + entity_conf) / 2.0
    }
    
    fn is_stop_word(&self, word: &str) -> bool {
        self.stop_words.contains(&word.to_lowercase())
    }
    
    fn is_method_keyword(&self, word: &str) -> bool {
        let methods = [
            "algorithm", "model", "network", "transformer", "cnn", "rnn", 
            "lstm", "bert", "gpt", "attention", "embedding", "classifier",
            "regression", "clustering", "detection", "segmentation",
        ];
        methods.contains(&word.to_lowercase().as_str())
    }
    
    fn is_temporal(&self, word: &str) -> bool {
        // Check for years or temporal terms
        if let Ok(year) = word.parse::<i32>() {
            return (1900..=2100).contains(&year);
        }
        let temporal_terms = ["recent", "latest", "new", "early", "current"];
        temporal_terms.contains(&word.to_lowercase().as_str())
    }
    
    fn is_known_concept(&self, bigram: &str) -> bool {
        let known = [
            "machine learning", "deep learning", "neural network", 
            "natural language", "computer vision", "reinforcement learning",
            "transfer learning", "attention mechanism", "language model",
            "knowledge graph", "graph neural", "generative model",
        ];
        known.contains(&bigram.to_lowercase().as_str())
    }
    
    fn load_default_synonyms() -> HashMap<String, Vec<String>> {
        let mut synonyms = HashMap::new();
        
        // Add some common ML/research synonyms
        synonyms.insert("ml".to_string(), vec!["machine learning".to_string()]);
        synonyms.insert("nlp".to_string(), vec!["natural language processing".to_string()]);
        synonyms.insert("cv".to_string(), vec!["computer vision".to_string()]);
        synonyms.insert("dl".to_string(), vec!["deep learning".to_string()]);
        synonyms.insert("llm".to_string(), vec!["large language model".to_string()]);
        synonyms.insert("rl".to_string(), vec!["reinforcement learning".to_string()]);
        synonyms.insert("gan".to_string(), vec!["generative adversarial network".to_string()]);
        synonyms.insert("vae".to_string(), vec!["variational autoencoder".to_string()]);
        
        synonyms
    }
    
    fn load_stop_words() -> Vec<String> {
        vec![
            "a", "an", "the", "is", "are", "was", "were", "be", "been",
            "in", "on", "at", "to", "for", "of", "with", "by", "from",
            "and", "or", "but", "not", "this", "that", "these", "those",
            "it", "its", "as", "do", "does", "did", "has", "have", "had",
            "can", "could", "will", "would", "should", "may", "might",
        ].into_iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_query_parsing() {
        let parser = QueryParser::new(QueryParserConfig::default());
        
        let result = parser.parse("How does transformer attention mechanism work?").await.unwrap();
        
        assert_eq!(result.intent, QueryIntent::Exploratory);
        assert!(!result.entities.is_empty());
    }
    
    #[tokio::test]
    async fn test_comparison_intent() {
        let parser = QueryParser::new(QueryParserConfig::default());
        
        let result = parser.parse("Compare BERT vs GPT for text classification").await.unwrap();
        
        assert_eq!(result.intent, QueryIntent::Comparison);
    }
    
    #[tokio::test]
    async fn test_procedural_intent() {
        let parser = QueryParser::new(QueryParserConfig::default());
        
        let result = parser.parse("How to implement attention mechanism").await.unwrap();
        
        assert_eq!(result.intent, QueryIntent::Procedural);
    }
}
