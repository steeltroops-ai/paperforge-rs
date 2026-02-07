mod config;
mod db;
mod embeddings;
mod errors;
mod routes;
mod services;
mod metrics;
mod middleware;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::EnvFilter;

/// Graceful shutdown signal handler
/// Listens for SIGINT (Ctrl+C) and SIGTERM
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT, initiating graceful shutdown...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown...");
        }
    }
    
    tracing::info!("Shutdown signal received, draining connections...");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load configuration
    dotenvy::dotenv().ok();
    let config = config::AppConfig::build().expect("Failed to load configuration");

    // 2. Setup logging with JSON format for production
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.server.rust_log))
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting PaperForge-rs..."
    );

    // 3. Initialize Database
    let repo = db::Repository::new(&config.database).await?;
    tracing::info!("Connected to database");

    // 4. Initialize Embeddings Service
    let embedder: Arc<dyn embeddings::Embedder> = if config.embeddings.model_api_key == "mock" {
        tracing::warn!("Using mock embedder - not for production use");
        Arc::new(embeddings::MockEmbedder::new(config.embeddings.embedding_dim))
    } else {
        Arc::new(embeddings::CloudEmbedder::new(config.embeddings.clone()))
    };

    // 5. Initialize App State (Services)
    let state = services::AppState::new(repo.clone(), embedder);

    // 6. Setup Router with middleware
    let app = routes::create_router(state, repo);

    // 7. Start Server with graceful shutdown
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!(address = %addr, "Server listening");
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}
