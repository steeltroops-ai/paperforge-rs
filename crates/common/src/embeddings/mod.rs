//! Embedding service abstraction
//!
//! Provides a unified interface for multiple embedding providers:
//! - OpenAI (text-embedding-ada-002, text-embedding-3-small)
//! - Anthropic
//! - Local models (e.g., E5, all-MiniLM)

use crate::errors::{AppError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Trait for embedding generation
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    
    /// Get the model name
    fn model_name(&self) -> &str;
    
    /// Get the embedding dimension
    fn dimension(&self) -> usize;
}

/// OpenAI embedding client
pub struct OpenAIEmbedder {
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimension: usize,
    base_url: String,
}

#[derive(Serialize)]
struct OpenAIRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    data: Vec<OpenAIEmbedding>,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
}

impl OpenAIEmbedder {
    /// Create a new OpenAI embedder
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "text-embedding-ada-002".to_string());
        let dimension = match model.as_str() {
            "text-embedding-ada-002" => 1536,
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            _ => 768,
        };
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key,
            model,
            dimension,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
    
    /// Make request with retry
    async fn request_with_retry(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let max_retries = 3;
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            if attempt > 0 {
                // Exponential backoff
                let delay = Duration::from_millis(100 * (2_u64.pow(attempt as u32)));
                tokio::time::sleep(delay).await;
            }
            
            match self.make_request(texts).await {
                Ok(embeddings) => return Ok(embeddings),
                Err(e) => {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        error = %e,
                        "Embedding request failed, retrying"
                    );
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| AppError::EmbeddingError {
            message: "Unknown error after retries".to_string(),
        }))
    }
    
    async fn make_request(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/embeddings", self.base_url);
        
        let request = OpenAIRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
        };
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::EmbeddingError {
                message: format!("Request failed: {}", e),
            })?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::EmbeddingError {
                message: format!("API error {}: {}", status, body),
            });
        }
        
        let result: OpenAIResponse = response.json().await.map_err(|e| {
            AppError::EmbeddingError {
                message: format!("Failed to parse response: {}", e),
            }
        })?;
        
        Ok(result.data.into_iter().map(|e| e.embedding).collect())
    }
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.request_with_retry(&[text.to_string()]).await?;
        embeddings.into_iter().next().ok_or_else(|| AppError::EmbeddingError {
            message: "Empty response".to_string(),
        })
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // OpenAI has a limit of 2048 texts per request
        const BATCH_SIZE: usize = 100;
        
        let mut all_embeddings = Vec::with_capacity(texts.len());
        
        for chunk in texts.chunks(BATCH_SIZE) {
            let embeddings = self.request_with_retry(chunk).await?;
            all_embeddings.extend(embeddings);
        }
        
        Ok(all_embeddings)
    }
    
    fn model_name(&self) -> &str {
        &self.model
    }
    
    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Mock embedder for testing
pub struct MockEmbedder {
    dimension: usize,
}

impl MockEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Ok((0..self.dimension).map(|_| rng.gen::<f32>()).collect())
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for _ in texts {
            embeddings.push(self.embed("").await?);
        }
        Ok(embeddings)
    }
    
    fn model_name(&self) -> &str {
        "mock-embedding"
    }
    
    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Create an embedder based on configuration
pub fn create_embedder(
    provider: &str,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
) -> Arc<dyn Embedder> {
    match provider {
        "openai" => {
            let key = api_key.expect("OpenAI API key required");
            Arc::new(OpenAIEmbedder::new(key, model, base_url))
        }
        "mock" => {
            Arc::new(MockEmbedder::new(768))
        }
        _ => {
            tracing::warn!(provider = provider, "Unknown embedding provider, using mock");
            Arc::new(MockEmbedder::new(768))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_embedder() {
        let embedder = MockEmbedder::new(768);
        let embedding = embedder.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), 768);
    }
    
    #[tokio::test]
    async fn test_mock_batch() {
        let embedder = MockEmbedder::new(768);
        let texts = vec!["text1".to_string(), "text2".to_string()];
        let embeddings = embedder.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768);
    }
}
