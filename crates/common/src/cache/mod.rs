//! Redis cache integration
//!
//! Provides:
//! - Connection pool management
//! - Generic get/set operations with TTL
//! - Query result caching
//! - Session storage

use crate::errors::{AppError, Result};
use redis::{AsyncCommands, Client, aio::MultiplexedConnection};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Redis cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis URL (redis://host:port)
    pub url: String,
    /// Default TTL in seconds
    pub default_ttl_secs: u64,
    /// Connection pool size
    pub pool_size: usize,
    /// Key prefix for namespacing
    pub key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            default_ttl_secs: 300,
            pool_size: 10,
            key_prefix: "paperforge".to_string(),
        }
    }
}

/// Redis cache client
pub struct Cache {
    client: Client,
    connection: RwLock<MultiplexedConnection>,
    config: CacheConfig,
}

impl Cache {
    /// Create a new cache client
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| AppError::CacheError { 
                message: format!("Failed to create Redis client: {}", e) 
            })?;
        
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to connect to Redis: {}", e),
            })?;
        
        Ok(Self {
            client,
            connection: RwLock::new(connection),
            config,
        })
    }
    
    /// Build a prefixed key
    fn key(&self, key: &str) -> String {
        format!("{}:{}", self.config.key_prefix, key)
    }
    
    /// Get a value from cache
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let full_key = self.key(key);
        let mut conn = self.connection.write().await;
        
        let value: Option<String> = conn.get(&full_key).await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to get key '{}': {}", full_key, e),
            })?;
        
        match value {
            Some(json) => {
                let parsed = serde_json::from_str(&json)
                    .map_err(|e| AppError::CacheError {
                        message: format!("Failed to parse cached value: {}", e),
                    })?;
                debug!(key = %full_key, "Cache hit");
                Ok(Some(parsed))
            }
            None => {
                debug!(key = %full_key, "Cache miss");
                Ok(None)
            }
        }
    }
    
    /// Set a value in cache with default TTL
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, self.config.default_ttl_secs).await
    }
    
    /// Set a value in cache with custom TTL
    pub async fn set_with_ttl<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let full_key = self.key(key);
        let json = serde_json::to_string(value)
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to serialize value: {}", e),
            })?;
        
        let mut conn = self.connection.write().await;
        conn.set_ex(&full_key, &json, ttl_secs)
            .await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to set key '{}': {}", full_key, e),
            })?;
        
        debug!(key = %full_key, ttl_secs, "Cache set");
        Ok(())
    }
    
    /// Delete a key from cache
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let full_key = self.key(key);
        let mut conn = self.connection.write().await;
        
        let deleted: i32 = conn.del(&full_key).await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to delete key '{}': {}", full_key, e),
            })?;
        
        debug!(key = %full_key, deleted = deleted > 0, "Cache delete");
        Ok(deleted > 0)
    }
    
    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let full_key = self.key(key);
        let mut conn = self.connection.write().await;
        
        let exists: bool = conn.exists(&full_key).await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to check key '{}': {}", full_key, e),
            })?;
        
        Ok(exists)
    }
    
    /// Get or set with a loader function
    pub async fn get_or_load<T, F, Fut>(
        &self,
        key: &str,
        ttl_secs: u64,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Try to get from cache first
        if let Some(cached) = self.get::<T>(key).await? {
            return Ok(cached);
        }
        
        // Load from source
        let value = loader().await?;
        
        // Cache the result
        if let Err(e) = self.set_with_ttl(key, &value, ttl_secs).await {
            warn!(error = %e, "Failed to cache value, continuing without cache");
        }
        
        Ok(value)
    }
    
    /// Ping Redis to check connectivity
    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.connection.write().await;
        redis::cmd("PING")
            .query_async::<String>(&mut *conn)
            .await
            .map_err(|e| AppError::CacheError {
                message: format!("Redis ping failed: {}", e),
            })?;
        Ok(())
    }
}

/// Cache key builder helpers
pub mod keys {
    use uuid::Uuid;
    
    /// Build a search query cache key
    pub fn search_query(tenant_id: Uuid, query_hash: &str, mode: &str) -> String {
        format!("search:{}:{}:{}", tenant_id, mode, query_hash)
    }
    
    /// Build a session cache key
    pub fn session(session_id: Uuid) -> String {
        format!("session:{}", session_id)
    }
    
    /// Build a paper cache key
    pub fn paper(paper_id: Uuid) -> String {
        format!("paper:{}", paper_id)
    }
    
    /// Build an embedding cache key
    pub fn embedding(text_hash: &str, model: &str) -> String {
        format!("embedding:{}:{}", model, text_hash)
    }
    
    /// Build a rate limit key
    pub fn rate_limit(tenant_id: Uuid, endpoint: &str) -> String {
        format!("ratelimit:{}:{}", tenant_id, endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_builders() {
        let tenant_id = uuid::Uuid::new_v4();
        let session_id = uuid::Uuid::new_v4();
        
        assert!(keys::search_query(tenant_id, "abc123", "hybrid").contains("search:"));
        assert!(keys::session(session_id).contains("session:"));
        assert!(keys::embedding("hash", "ada-002").contains("embedding:"));
    }
}
