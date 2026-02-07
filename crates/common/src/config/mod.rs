//! Configuration management for PaperForge services
//!
//! Supports loading configuration from:
//! - Environment variables (prefixed with APP__)
//! - Configuration files (config.toml, config.yaml)
//! - Default values

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Redis configuration
    pub redis: RedisConfig,
    
    /// Embedding service configuration
    pub embedding: EmbeddingConfig,
    
    /// Queue configuration (SQS)
    pub queue: QueueConfig,
    
    /// Authentication configuration
    pub auth: AuthConfig,
    
    /// Observability configuration
    pub observability: ObservabilityConfig,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,
    
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,
    
    /// Shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
    
    /// Maximum concurrent requests
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Primary database URL (for writes)
    pub url: String,
    
    /// Read replica URL (optional, falls back to primary)
    pub read_url: Option<String>,
    
    /// Maximum number of connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    /// Minimum number of connections
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    
    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,
    
    /// Idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,
    
    /// Pool size
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: u32,
    
    /// Default TTL in seconds
    #[serde(default = "default_redis_ttl")]
    pub default_ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingConfig {
    /// Embedding provider: openai, anthropic, local
    #[serde(default = "default_embedding_provider")]
    pub provider: String,
    
    /// API key for embedding service
    pub api_key: Option<String>,
    
    /// API base URL (for custom endpoints)
    pub api_base: Option<String>,
    
    /// Model to use
    #[serde(default = "default_embedding_model")]
    pub model: String,
    
    /// Embedding dimension
    #[serde(default = "default_embedding_dimension")]
    pub dimension: usize,
    
    /// Request timeout in seconds
    #[serde(default = "default_embedding_timeout")]
    pub timeout_secs: u64,
    
    /// Maximum retries
    #[serde(default = "default_embedding_retries")]
    pub max_retries: u32,
    
    /// Batch size for embedding requests
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueueConfig {
    /// SQS ingestion queue URL
    pub ingestion_queue_url: Option<String>,
    
    /// SQS embedding queue URL
    pub embedding_queue_url: Option<String>,
    
    /// Dead letter queue URL
    pub dlq_url: Option<String>,
    
    /// Maximum messages to receive per poll
    #[serde(default = "default_queue_batch_size")]
    pub batch_size: u32,
    
    /// Long polling timeout in seconds
    #[serde(default = "default_queue_poll_timeout")]
    pub poll_timeout_secs: u64,
    
    /// Visibility timeout in seconds
    #[serde(default = "default_visibility_timeout")]
    pub visibility_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    /// JWT secret for token signing
    pub jwt_secret: Option<String>,
    
    /// JWT expiration in seconds
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_secs: u64,
    
    /// API key header name
    #[serde(default = "default_api_key_header")]
    pub api_key_header: String,
    
    /// Tenant ID header name
    #[serde(default = "default_tenant_header")]
    pub tenant_header: String,
    
    /// Request ID header name
    #[serde(default = "default_request_id_header")]
    pub request_id_header: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObservabilityConfig {
    /// Log level (debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
    
    /// Enable JSON logging
    #[serde(default = "default_json_logging")]
    pub json_logging: bool,
    
    /// OpenTelemetry endpoint
    pub otel_endpoint: Option<String>,
    
    /// Metrics port (0 to disable)
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    
    /// Service name for tracing
    #[serde(default = "default_service_name")]
    pub service_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    /// Requests per second (per tenant)
    #[serde(default = "default_rate_limit")]
    pub requests_per_second: u32,
    
    /// Burst capacity
    #[serde(default = "default_burst")]
    pub burst: u32,
    
    /// Enable rate limiting
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

// Default value functions
fn default_host() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 8080 }
fn default_request_timeout() -> u64 { 30 }
fn default_shutdown_timeout() -> u64 { 30 }
fn default_max_concurrent() -> usize { 100 }
fn default_max_connections() -> u32 { 50 }
fn default_min_connections() -> u32 { 5 }
fn default_connect_timeout() -> u64 { 10 }
fn default_idle_timeout() -> u64 { 300 }
fn default_redis_pool_size() -> u32 { 20 }
fn default_redis_ttl() -> u64 { 300 }
fn default_embedding_provider() -> String { "openai".to_string() }
fn default_embedding_model() -> String { "text-embedding-ada-002".to_string() }
fn default_embedding_dimension() -> usize { 768 }
fn default_embedding_timeout() -> u64 { 30 }
fn default_embedding_retries() -> u32 { 3 }
fn default_batch_size() -> usize { 10 }
fn default_queue_batch_size() -> u32 { 10 }
fn default_queue_poll_timeout() -> u64 { 20 }
fn default_visibility_timeout() -> u64 { 300 }
fn default_jwt_expiration() -> u64 { 3600 }
fn default_api_key_header() -> String { "Authorization".to_string() }
fn default_tenant_header() -> String { "X-Tenant-ID".to_string() }
fn default_request_id_header() -> String { "X-Request-ID".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_json_logging() -> bool { true }
fn default_metrics_port() -> u16 { 9090 }
fn default_service_name() -> String { "paperforge".to_string() }
fn default_rate_limit() -> u32 { 50 }
fn default_burst() -> u32 { 100 }
fn default_enabled() -> bool { true }

impl AppConfig {
    /// Load configuration from environment and files
    pub fn load() -> Result<Self, ConfigError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        
        let config = Config::builder()
            // Start with defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            
            // Load base config file
            .add_source(File::with_name("config/default").required(false))
            
            // Load environment-specific config
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            
            // Load local overrides
            .add_source(File::with_name("config/local").required(false))
            
            // Load from environment variables with APP__ prefix
            // e.g., APP__SERVER__PORT=8081
            .add_source(
                Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true)
            )
            
            .build()?;
            
        config.try_deserialize()
    }
    
    /// Load from a specific TOML file
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name(path))
            .add_source(
                Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true)
            )
            .build()?;
            
        config.try_deserialize()
    }
    
    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.server.request_timeout_secs)
    }
    
    /// Get shutdown timeout as Duration
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.server.shutdown_timeout_secs)
    }
    
    /// Get the read database URL (falls back to primary)
    pub fn read_database_url(&self) -> &str {
        self.database.read_url.as_deref().unwrap_or(&self.database.url)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
                request_timeout_secs: default_request_timeout(),
                shutdown_timeout_secs: default_shutdown_timeout(),
                max_concurrent_requests: default_max_concurrent(),
            },
            database: DatabaseConfig {
                url: "postgres://localhost/paperforge".to_string(),
                read_url: None,
                max_connections: default_max_connections(),
                min_connections: default_min_connections(),
                connect_timeout_secs: default_connect_timeout(),
                idle_timeout_secs: default_idle_timeout(),
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: default_redis_pool_size(),
                default_ttl_secs: default_redis_ttl(),
            },
            embedding: EmbeddingConfig {
                provider: default_embedding_provider(),
                api_key: None,
                api_base: None,
                model: default_embedding_model(),
                dimension: default_embedding_dimension(),
                timeout_secs: default_embedding_timeout(),
                max_retries: default_embedding_retries(),
                batch_size: default_batch_size(),
            },
            queue: QueueConfig {
                ingestion_queue_url: None,
                embedding_queue_url: None,
                dlq_url: None,
                batch_size: default_queue_batch_size(),
                poll_timeout_secs: default_queue_poll_timeout(),
                visibility_timeout_secs: default_visibility_timeout(),
            },
            auth: AuthConfig {
                jwt_secret: None,
                jwt_expiration_secs: default_jwt_expiration(),
                api_key_header: default_api_key_header(),
                tenant_header: default_tenant_header(),
                request_id_header: default_request_id_header(),
            },
            observability: ObservabilityConfig {
                log_level: default_log_level(),
                json_logging: default_json_logging(),
                otel_endpoint: None,
                metrics_port: default_metrics_port(),
                service_name: default_service_name(),
            },
            rate_limit: RateLimitConfig {
                requests_per_second: default_rate_limit(),
                burst: default_burst(),
                enabled: default_enabled(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.embedding.model, "text-embedding-ada-002");
    }
    
    #[test]
    fn test_read_database_fallback() {
        let config = AppConfig::default();
        assert_eq!(config.read_database_url(), "postgres://localhost/paperforge");
    }
}
