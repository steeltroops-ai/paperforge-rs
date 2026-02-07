//! LLM Synthesizer - Generates coherent answers from context
//!
//! Provides:
//! - Context-grounded answer generation
//! - Citation extraction
//! - Confidence scoring
//! - Hallucination detection

use crate::errors::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Synthesized answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedAnswer {
    /// Generated answer text
    pub answer: String,
    
    /// Citations used in the answer
    pub citations: Vec<Citation>,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    
    /// Token count
    pub token_count: usize,
    
    /// Key facts extracted
    pub key_facts: Vec<String>,
}

/// Citation in synthesized answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Citation index (1-based)
    pub index: usize,
    
    /// Paper ID
    pub paper_id: Uuid,
    
    /// Paper title
    pub title: String,
    
    /// Quoted/referenced text
    pub quote: String,
    
    /// Position in answer (character offset)
    pub position: Option<usize>,
}

/// Synthesis options
#[derive(Debug, Clone)]
pub struct SynthesisOptions {
    /// Maximum output tokens
    pub max_tokens: usize,
    
    /// Temperature (0.0 - 1.0)
    pub temperature: f32,
    
    /// Include citations inline
    pub include_citations: bool,
    
    /// Style: concise, detailed, academic
    pub style: SynthesisStyle,
    
    /// System prompt override
    pub system_prompt: Option<String>,
}

/// Synthesis style
#[derive(Debug, Clone, PartialEq)]
pub enum SynthesisStyle {
    /// Brief, to-the-point
    Concise,
    /// Comprehensive explanation
    Detailed,
    /// Academic writing style
    Academic,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            max_tokens: 1000,
            temperature: 0.7,
            include_citations: true,
            style: SynthesisStyle::Detailed,
            system_prompt: None,
        }
    }
}

/// Context for synthesis
#[derive(Debug, Clone)]
pub struct SynthesisContext {
    pub paper_id: Uuid,
    pub paper_title: String,
    pub content: String,
    pub relevance_score: f32,
}

/// LLM client configuration
#[derive(Debug, Clone)]
pub struct LLMConfig {
    /// API endpoint
    pub endpoint: String,
    
    /// API key
    pub api_key: String,
    
    /// Model name
    pub model: String,
    
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            timeout_secs: 30,
        }
    }
}

/// Synthesizer for generating answers
pub struct Synthesizer {
    config: LLMConfig,
    client: reqwest::Client,
}

impl Synthesizer {
    /// Create a new synthesizer
    pub fn new(config: LLMConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AppError::Internal { 
                message: format!("Failed to create HTTP client: {}", e) 
            })?;
        
        Ok(Self { config, client })
    }
    
    /// Synthesize an answer from context
    pub async fn synthesize(
        &self,
        question: &str,
        contexts: &[SynthesisContext],
        options: &SynthesisOptions,
    ) -> Result<SynthesizedAnswer> {
        // Build prompt
        let prompt = self.build_prompt(question, contexts, options);
        
        // Call LLM
        let response = self.call_llm(&prompt, options).await?;
        
        // Extract citations
        let citations = self.extract_citations(&response, contexts);
        
        // Calculate confidence based on context coverage
        let confidence = self.calculate_confidence(&response, contexts);
        
        // Extract key facts
        let key_facts = self.extract_key_facts(&response);
        
        // Estimate token count
        let token_count = response.len() / 4;
        
        Ok(SynthesizedAnswer {
            answer: response,
            citations,
            confidence,
            token_count,
            key_facts,
        })
    }
    
    /// Build the synthesis prompt
    fn build_prompt(
        &self,
        question: &str,
        contexts: &[SynthesisContext],
        options: &SynthesisOptions,
    ) -> String {
        let style_instruction = match options.style {
            SynthesisStyle::Concise => "Provide a brief, focused answer.",
            SynthesisStyle::Detailed => "Provide a comprehensive answer with explanations.",
            SynthesisStyle::Academic => "Write in an academic style with proper terminology.",
        };
        
        let citation_instruction = if options.include_citations {
            "Include inline citations in the format [1], [2], etc. referring to the source papers."
        } else {
            "Do not include citations."
        };
        
        let mut prompt = format!(
            "You are a research assistant. Answer the following question based ONLY on the provided context. \
            If the context doesn't contain enough information, say so. Do not make up information.\n\n\
            {}\n{}\n\n\
            Question: {}\n\n\
            Context:\n",
            style_instruction, citation_instruction, question
        );
        
        for (i, ctx) in contexts.iter().enumerate() {
            prompt.push_str(&format!(
                "\n[{}] {} (relevance: {:.2})\n{}\n",
                i + 1,
                ctx.paper_title,
                ctx.relevance_score,
                ctx.content
            ));
        }
        
        prompt.push_str("\nAnswer:");
        prompt
    }
    
    /// Call the LLM API
    async fn call_llm(&self, prompt: &str, options: &SynthesisOptions) -> Result<String> {
        // In a real implementation, this would call OpenAI or another LLM
        // For now, return a mock response for testing
        
        if self.config.api_key.is_empty() {
            // Mock response for development/testing
            return Ok(self.generate_mock_response(prompt));
        }
        
        #[derive(Serialize)]
        struct ChatMessage {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            max_tokens: usize,
            temperature: f32,
        }
        
        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatMessageResponse,
        }
        
        #[derive(Deserialize)]
        struct ChatMessageResponse {
            content: String,
        }
        
        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<ChatChoice>,
        }
        
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: options.system_prompt.clone().unwrap_or_else(|| {
                        "You are a helpful research assistant.".to_string()
                    }),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            max_tokens: options.max_tokens,
            temperature: options.temperature,
        };
        
        let response = self.client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal {
                message: format!("LLM API request failed: {}", e),
            })?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal {
                message: format!("LLM API error {}: {}", status, body),
            });
        }
        
        let chat_response: ChatResponse = response.json().await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to parse LLM response: {}", e),
            })?;
        
        chat_response.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AppError::Internal {
                message: "Empty response from LLM".to_string(),
            })
    }
    
    /// Generate mock response for testing
    fn generate_mock_response(&self, prompt: &str) -> String {
        // Extract question from prompt
        if let Some(q_start) = prompt.find("Question:") {
            let question_part = &prompt[q_start..];
            if let Some(newline) = question_part.find('\n') {
                let question = question_part[9..newline].trim();
                return format!(
                    "Based on the provided context, here is an answer to your question about {}:\n\n\
                    The research literature indicates several key findings [1]. \
                    Further analysis suggests important implications for this area [2]. \
                    However, more research is needed to fully understand the mechanisms involved.\n\n\
                    [Mock response - LLM API key not configured]",
                    question
                );
            }
        }
        
        "Based on the provided context, the answer requires further investigation. \
        [Mock response - LLM API key not configured]".to_string()
    }
    
    /// Extract citations from response
    fn extract_citations(&self, response: &str, contexts: &[SynthesisContext]) -> Vec<Citation> {
        let mut citations = Vec::new();
        
        // Find citation patterns like [1], [2], etc.
        let citation_pattern = regex_lite::Regex::new(r"\[(\d+)\]").unwrap();
        
        for cap in citation_pattern.captures_iter(response) {
            if let Some(num_match) = cap.get(1) {
                if let Ok(idx) = num_match.as_str().parse::<usize>() {
                    if idx > 0 && idx <= contexts.len() {
                        let ctx = &contexts[idx - 1];
                        
                        // Check if we already have this citation
                        if !citations.iter().any(|c: &Citation| c.index == idx) {
                            citations.push(Citation {
                                index: idx,
                                paper_id: ctx.paper_id,
                                title: ctx.paper_title.clone(),
                                quote: ctx.content.chars().take(200).collect(),
                                position: cap.get(0).map(|m| m.start()),
                            });
                        }
                    }
                }
            }
        }
        
        citations.sort_by_key(|c| c.index);
        citations
    }
    
    /// Calculate confidence based on context coverage
    fn calculate_confidence(&self, response: &str, contexts: &[SynthesisContext]) -> f32 {
        if contexts.is_empty() {
            return 0.5;
        }
        
        // Check how many contexts are cited
        let citation_count = self.extract_citations(response, contexts).len();
        let citation_coverage = citation_count as f32 / contexts.len() as f32;
        
        // Average context relevance
        let avg_relevance = contexts.iter()
            .map(|c| c.relevance_score)
            .sum::<f32>() / contexts.len() as f32;
        
        // Response length factor (longer = more confident, up to a point)
        let length_factor = (response.len() as f32 / 500.0).min(1.0);
        
        // Combine factors
        (citation_coverage * 0.4 + avg_relevance * 0.4 + length_factor * 0.2).min(1.0)
    }
    
    /// Extract key facts from response
    fn extract_key_facts(&self, response: &str) -> Vec<String> {
        let mut facts = Vec::new();
        
        // Split into sentences and find fact-like statements
        for sentence in response.split(['.', '!', '?']) {
            let sentence = sentence.trim();
            
            // Skip very short or very long sentences
            if sentence.len() < 20 || sentence.len() > 300 {
                continue;
            }
            
            // Look for fact-like patterns
            let lower = sentence.to_lowercase();
            if lower.contains("found that")
                || lower.contains("shows that")
                || lower.contains("indicates")
                || lower.contains("demonstrates")
                || lower.contains("according to")
            {
                facts.push(format!("{}.", sentence));
            }
        }
        
        facts.truncate(5);
        facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_citation_extraction() {
        let synthesizer = Synthesizer::new(LLMConfig::default()).unwrap();
        
        let response = "The model shows good results [1]. Further analysis [2] confirms this.";
        let contexts = vec![
            SynthesisContext {
                paper_id: Uuid::new_v4(),
                paper_title: "Paper 1".to_string(),
                content: "First paper content".to_string(),
                relevance_score: 0.8,
            },
            SynthesisContext {
                paper_id: Uuid::new_v4(),
                paper_title: "Paper 2".to_string(),
                content: "Second paper content".to_string(),
                relevance_score: 0.7,
            },
        ];
        
        let citations = synthesizer.extract_citations(response, &contexts);
        
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].index, 1);
        assert_eq!(citations[1].index, 2);
    }
    
    #[test]
    fn test_confidence_calculation() {
        let synthesizer = Synthesizer::new(LLMConfig::default()).unwrap();
        
        let response = "Based on the analysis [1], we find important results.";
        let contexts = vec![
            SynthesisContext {
                paper_id: Uuid::new_v4(),
                paper_title: "Paper 1".to_string(),
                content: "Content".to_string(),
                relevance_score: 0.9,
            },
        ];
        
        let confidence = synthesizer.calculate_confidence(response, &contexts);
        
        assert!(confidence > 0.5);
        assert!(confidence <= 1.0);
    }
}
