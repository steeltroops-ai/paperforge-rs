use serde::Deserialize;
use config::{Config, ConfigError, File, Environment};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub embeddings: EmbeddingsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub rust_log: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmbeddingsConfig {
    pub model_api_url: String, // e.g. OpenAI or local TEI
    pub model_api_key: String,
    pub embedding_dim: usize,
}

impl AppConfig {
    pub fn build() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let builder = Config::builder()
            // Start with defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("server.rust_log", "info,paperforge_rs=debug")?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 2)?
            .set_default("database.connect_timeout", 30)?
            .set_default("embeddings.embedding_dim", 768)?
            // Add in settings from files (optional)
            // .add_source(File::with_name("config/default"))
            // .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Add in settings from environment variables (with a prefix of APP)
            // E.g. `APP_SERVER__PORT=8080` would set `ServerConfig.port`
            .add_source(Environment::default().separator("__").prefix("APP"));

        builder.build()?.try_deserialize()
    }
}
