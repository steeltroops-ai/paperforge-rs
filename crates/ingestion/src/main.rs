//! PaperForge Ingestion Service
//!
//! Processes ingestion jobs from SQS queue:
//! 1. Receives job message
//! 2. Chunks the paper text
//! 3. Sends chunks to embedding queue
//! 4. Updates job status

use paperforge_common::{config::AppConfig, db::DbPool, VERSION};
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
    
    info!("Starting PaperForge Ingestion Service v{}", VERSION);
    
    // Load configuration
    let config = AppConfig::load().map_err(|e| {
        tracing::error!(error = %e, "Failed to load configuration");
        e
    })?;
    
    let config = Arc::new(config);
    
    // Initialize database connection
    info!("Connecting to database...");
    let _db = DbPool::new(&config.database).await?;
    
    // TODO: Initialize SQS client
    // TODO: Start polling loop
    
    info!("Ingestion service ready (placeholder implementation)");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    info!("Ingestion service shutting down");
    Ok(())
}
