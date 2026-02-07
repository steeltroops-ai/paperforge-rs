use async_trait::async_trait;
use crate::config::EmbeddingsConfig;
use crate::errors::AppError;
use std::time::Duration;

/// Request timeout for embedding API calls
const EMBEDDING_TIMEOUT_SECS: u64 = 30;

/// Maximum retries for transient failures
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (ms)
const RETRY_BASE_DELAY_MS: u64 = 100;

/// Trait for embedding generation
/// 
/// Implementations must be Send + Sync for use across tokio tasks
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate embedding for a single query
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError>;
    
    /// Generate embeddings for multiple documents
    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError>;
    
    /// Health check for the embedding service
    async fn health(&self) -> Result<(), AppError> {
        // Default implementation - try to embed a test string
        self.embed_query("health check").await.map(|_| ())
    }
}

/// Cloud-based embedder using external API (e.g., OpenAI)
pub struct CloudEmbedder {
    client: reqwest::Client,
    config: EmbeddingsConfig,
}

impl CloudEmbedder {
    pub fn new(config: EmbeddingsConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(EMBEDDING_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        
        Self { client, config }
    }
    
    /// Execute request with retry logic
    async fn request_with_retry<T, F, Fut>(&self, operation: F) -> Result<T, AppError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        let mut last_error = AppError::EmbeddingError("Unknown error".to_string());
        
        for attempt in 0..MAX_RETRIES {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = e;
                    
                    // Don't retry on non-transient errors
                    if matches!(&last_error, 
                        AppError::ValidationError(_) | 
                        AppError::InvalidApiKey
                    ) {
                        return Err(last_error);
                    }
                    
                    if attempt < MAX_RETRIES - 1 {
                        // Exponential backoff with jitter
                        let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                        let jitter = rand::random::<u64>() % (delay / 2);
                        
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_retries = MAX_RETRIES,
                            delay_ms = delay + jitter,
                            error = %last_error,
                            "Embedding request failed, retrying"
                        );
                        
                        tokio::time::sleep(Duration::from_millis(delay + jitter)).await;
                    }
                }
            }
        }
        
        Err(last_error)
    }
}

#[async_trait]
impl Embedder for CloudEmbedder {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let text = text.to_string();
        let config = self.config.clone();
        let client = self.client.clone();
        
        self.request_with_retry(|| {
            let text = text.clone();
            let config = config.clone();
            let client = client.clone();
            
            async move {
                let payload = serde_json::json!({
                    "input": text,
                    "model": "text-embedding-ada-002"
                });

                let res = client
                    .post(&config.model_api_url)
                    .header("Authorization", format!("Bearer {}", config.model_api_key))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            AppError::EmbeddingServiceTimeout { timeout_secs: EMBEDDING_TIMEOUT_SECS }
                        } else if e.is_connect() {
                            AppError::EmbeddingServiceUnavailable(format!("Connection failed: {}", e))
                        } else {
                            AppError::EmbeddingError(format!("Request failed: {}", e))
                        }
                    })?;

                let status = res.status();
                
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(AppError::InvalidApiKey);
                }
                
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(AppError::RateLimitExceeded { retry_after_secs: 60 });
                }
                
                if !status.is_success() {
                    let error_body = res.text().await.unwrap_or_default();
                    return Err(AppError::EmbeddingError(
                        format!("API error {}: {}", status, error_body)
                    ));
                }

                let body: serde_json::Value = res.json().await
                    .map_err(|e| AppError::EmbeddingError(format!("Parse error: {}", e)))?;

                let embedding = body["data"][0]["embedding"]
                    .as_array()
                    .ok_or_else(|| AppError::EmbeddingError("Invalid response format".to_string()))?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect();

                Ok(embedding)
            }
        }).await
    }

    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        // For better performance, this should use batch API
        // For MVP, we process sequentially with proper error handling
        let mut results = Vec::with_capacity(texts.len());
        
        for (i, text) in texts.iter().enumerate() {
            match self.embed_query(text).await {
                Ok(embedding) => results.push(embedding),
                Err(e) => {
                    tracing::error!(
                        chunk_index = i,
                        total_chunks = texts.len(),
                        error = %e,
                        "Failed to embed chunk"
                    );
                    return Err(e);
                }
            }
        }
        
        Ok(results)
    }
}

/// Mock embedder for testing and development
pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError> {
        // Generate deterministic embeddings based on text hash
        // This makes tests reproducible
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();
        
        // Generate normalized vector
        let mut embedding: Vec<f32> = (0..self.dim)
            .map(|i| {
                // Simple PRNG based on seed and index
                let x = ((seed.wrapping_mul(i as u64 + 1)) % 1000) as f32 / 1000.0;
                x * 2.0 - 1.0 // Range [-1, 1]
            })
            .collect();
        
        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }
        
        Ok(embedding)
    }

    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed_query(&text).await?);
        }
        Ok(results)
    }
    
    async fn health(&self) -> Result<(), AppError> {
        // Mock embedder is always healthy
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_embedder_deterministic() {
        let embedder = MockEmbedder::new(768);
        
        let emb1 = embedder.embed_query("test").await.unwrap();
        let emb2 = embedder.embed_query("test").await.unwrap();
        
        assert_eq!(emb1, emb2);
        assert_eq!(emb1.len(), 768);
    }
    
    #[tokio::test]
    async fn test_mock_embedder_different_texts() {
        let embedder = MockEmbedder::new(768);
        
        let emb1 = embedder.embed_query("hello").await.unwrap();
        let emb2 = embedder.embed_query("world").await.unwrap();
        
        assert_ne!(emb1, emb2);
    }
    
    #[tokio::test]
    async fn test_mock_embedder_normalized() {
        let embedder = MockEmbedder::new(768);
        let emb = embedder.embed_query("test").await.unwrap();
        
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }
}
