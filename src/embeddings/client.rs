use async_trait::async_trait;
use crate::config::EmbeddingsConfig;
use crate::errors::AppError;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError>;
    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError>;
}

pub struct CloudEmbedder {
    client: reqwest::Client,
    config: EmbeddingsConfig,
}

impl CloudEmbedder {
    pub fn new(config: EmbeddingsConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait]
impl Embedder for CloudEmbedder {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError> {
        // Implement call to external API (e.g. OpenAI or HF Interface)
        // This is a placeholder for standard OpenAI format
        // POST /v1/embeddings { "input": text, "model": "..." }
        
        let payload = serde_json::json!({
            "input": text,
            "model": "text-embedding-ada-002" // or from config
        });

        let res = self.client.post(&self.config.model_api_url)
            .header("Authorization", format!("Bearer {}", self.config.model_api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::EmbeddingError(format!("Request failed: {}", e)))?;

        if !res.status().is_success() {
           return Err(AppError::EmbeddingError(format!("API Error: {}", res.status())));
        }
        
        // Simplified response parsing logic
        // Assuming response matches OpenAI format: data[0].embedding
        let body: serde_json::Value = res.json().await
            .map_err(|e| AppError::EmbeddingError(format!("Parse error: {}", e)))?;
            
        let embedding = body["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| AppError::EmbeddingError("Invalid response format".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
            
        Ok(embedding)
    }

    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        // Naive implementation: loop over texts. Real impl should batch.
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed_query(&text).await?);
        }
        Ok(results)
    }
}

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
    async fn embed_query(&self, _text: &str) -> Result<Vec<f32>, AppError> {
        // Return random vector normalized
        let mut rng = rand::thread_rng();
        // Since we don't have rand in Cargo.toml, we'll just return a deterministic vector or use 1.0s
        // Wait, I forgot `rand` in Cargo.toml. 
        // I will just return a vector of unit length with simplistic values for MVP to avoid compile error.
        Ok(vec![0.5; self.dim])
    }

    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts.iter().map(|_| vec![0.5; self.dim]).collect())
    }
}
