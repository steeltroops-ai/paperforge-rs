//! PaperForge Embedding Worker
//!
//! Processes embedding jobs from SQS queue:
//! 1. Receives chunk batch from queue
//! 2. Generates embeddings via OpenAI/local model
//! 3. Writes embeddings to database
//! 4. Updates job progress

use paperforge_common::{
    config::AppConfig, 
    db::DbPool, 
    embeddings::{create_embedder, Embedder},
    VERSION
};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(true)
        .json()
        .init();
    
    info!("Starting PaperForge Embedding Worker v{}", VERSION);
    
    // Load configuration
    let config = AppConfig::load().map_err(|e| {
        tracing::error!(error = %e, "Failed to load configuration");
        e
    })?;
    
    let config = Arc::new(config);
    
    // Initialize database connection
    info!("Connecting to database...");
    let _db = DbPool::new(&config.database).await?;
    
    // Initialize embedder
    let embedder = create_embedder(
        &config.embedding.provider,
        config.embedding.api_key.clone(),
        Some(config.embedding.model.clone()),
        config.embedding.api_base.clone(),
    );
    
    info!(
        model = %embedder.model_name(),
        dimension = embedder.dimension(),
        "Embedder initialized"
    );
    
    // TODO: Initialize SQS client
    // TODO: Start polling loop with circuit breaker
    
    info!("Embedding Worker ready (placeholder implementation)");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    info!("Embedding Worker shutting down");
    Ok(())
}
