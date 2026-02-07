mod config;
mod db;
mod embeddings;
mod errors;
mod routes;
mod services;
mod metrics;

use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load configuration
    dotenvy::dotenv().ok();
    let config = config::AppConfig::build().expect("Failed to load configuration");

    // 2. Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.server.rust_log))
        .init();

    tracing::info!("Starting PaperForge-rs...");

    // 3. Initialize Database
    let repo = db::Repository::new(&config.database).await?;
    tracing::info!("Connected to database");

    // 4. Initialize Embeddings Service
    // For MVP, we use MockEmbedder if API key is "mock", else CloudEmbedder
    let embedder: Arc<dyn embeddings::Embedder> = if config.embeddings.model_api_key == "mock" {
        Arc::new(embeddings::MockEmbedder::new(config.embeddings.embedding_dim))
    } else {
        Arc::new(embeddings::CloudEmbedder::new(config.embeddings.clone()))
    };

    // 5. Initialize App State (Services)
    let state = services::AppState::new(repo, embedder);

    // 6. Setup Router
    let app = routes::create_router(state);

    // 7. Start Server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
