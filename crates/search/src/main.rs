//! PaperForge Search Service
//!
//! Dedicated search microservice providing:
//! - Vector similarity search (pgvector)
//! - BM25 text search (PostgreSQL full-text)
//! - Hybrid search with RRF fusion
//! - Citation graph traversal & PageRank scoring
//! - Query caching via Redis

mod retrieval;
mod citation;
mod grpc;

use paperforge_common::{config::AppConfig, db::DbPool, cache::{Cache, CacheConfig}, VERSION};
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::{info, warn, Level};

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
    
    info!("Starting PaperForge Search Service v{}", VERSION);
    
    // Load configuration
    let config = AppConfig::load().map_err(|e| {
        tracing::error!(error = %e, "Failed to load configuration");
        e
    })?;
    
    let config = Arc::new(config);
    
    // Initialize database connection
    info!("Connecting to database...");
    let db = Arc::new(DbPool::new(&config.database).await?);
    
    // Initialize Redis cache (optional)
    let cache = match std::env::var("REDIS_URL") {
        Ok(url) => {
            info!("Connecting to Redis at {}", url);
            let cache_config = CacheConfig {
                url,
                default_ttl_secs: 300,
                pool_size: 10,
                key_prefix: "paperforge:search".to_string(),
            };
            match Cache::new(cache_config).await {
                Ok(cache) => {
                    info!("Redis cache connected");
                    Some(Arc::new(cache))
                }
                Err(e) => {
                    warn!("Failed to connect to Redis, caching disabled: {}", e);
                    None
                }
            }
        }
        Err(_) => {
            warn!("REDIS_URL not set, caching disabled");
            None
        }
    };
    
    // Create gRPC service
    let search_service = grpc::SearchGrpcService::new(db, cache);
    
    // Get gRPC port
    let grpc_port = std::env::var("GRPC_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(50051);
    
    let addr: SocketAddr = ([0, 0, 0, 0], grpc_port).into();
    
    info!("Search service listening on gRPC port {}", grpc_port);
    
    // Start gRPC server
    Server::builder()
        .add_service(search_service.into_server())
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;
    
    info!("Search service shutdown complete");
    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, starting shutdown..."),
        _ = terminate => info!("Received SIGTERM, starting shutdown..."),
    }
}
