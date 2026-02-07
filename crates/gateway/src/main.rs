//! PaperForge API Gateway
//!
//! The main entry point for all external API requests.
//! Handles:
//! - Authentication and authorization
//! - Rate limiting
//! - Request routing
//! - Observability (logging, metrics, tracing)

mod handlers;
mod middleware;

use axum::{
    routing::{delete, get, post},
    Router,
};
use paperforge_common::{
    config::AppConfig,
    db::DbPool,
    errors::AppError,
    metrics,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: DbPool,
}

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
    
    info!("Starting PaperForge API Gateway v{}", paperforge_common::VERSION);
    
    // Load configuration
    let config = AppConfig::load().map_err(|e| {
        tracing::error!(error = %e, "Failed to load configuration");
        e
    })?;
    
    let config = Arc::new(config);
    
    // Initialize metrics
    metrics::register_metrics();
    
    // Initialize database connection
    info!("Connecting to database...");
    let db = DbPool::new(&config.database).await?;
    
    // Create app state
    let state = AppState {
        config: config.clone(),
        db,
    };
    
    // Build the router
    let app = create_router(state);
    
    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    info!("Server shutdown complete");
    Ok(())
}

/// Create the main application router
fn create_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Request ID propagation
    let request_id = SetRequestIdLayer::x_request_id(MakeRequestUuid);
    let propagate_id = PropagateRequestIdLayer::x_request_id();
    
    // API routes
    let api_routes = Router::new()
        // Health endpoints (no auth)
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        
        // Paper endpoints
        .route("/papers", post(handlers::papers::create_paper))
        .route("/papers/:id", get(handlers::papers::get_paper))
        .route("/papers/:id", delete(handlers::papers::delete_paper))
        
        // Job endpoints
        .route("/jobs/:id", get(handlers::jobs::get_job))
        
        // Search endpoints
        .route("/search", post(handlers::search::search))
        .route("/search/batch", post(handlers::search::batch_search))
        
        // Intelligence endpoints (Context Engine)
        .route("/intelligence/search", post(handlers::intelligence::intelligent_search))
        
        // Session endpoints
        .route("/sessions", post(handlers::sessions::create_session))
        .route("/sessions/:id", get(handlers::sessions::get_session))
        .route("/sessions/:id/events", post(handlers::sessions::track_event))
        
        // Citation endpoints
        .route("/papers/:id/citations", get(handlers::citations::get_citations))
        .route("/citations/traverse", post(handlers::citations::traverse_citations));
    
    // Compose the app
    Router::new()
        .nest("/v2", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(request_id)
        .layer(propagate_id)
        .with_state(state)
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
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
