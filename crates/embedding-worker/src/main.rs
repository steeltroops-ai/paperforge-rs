//! PaperForge Embedding Worker
//!
//! Processes embedding jobs from SQS queue:
//! 1. Receives chunk batch from queue
//! 2. Generates embeddings via OpenAI/local model
//! 3. Writes embeddings to database
//! 4. Updates job progress

mod processor;

use crate::processor::{EmbeddingConfig, EmbeddingJob, EmbeddingProcessor};
use paperforge_common::{
    config::AppConfig,
    db::DbPool,
    embeddings::{create_embedder, Embedder},
    queue::{Queue, QueueConfig},
    VERSION,
};
use std::sync::Arc;
use tracing::{error, info, warn, Level};

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
    let db = DbPool::new(&config.database).await?;

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

    // Initialize processor
    let processor = EmbeddingProcessor::new(db, embedder, EmbeddingConfig::default());

    // Check for command line arguments for testing
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "test" {
        // Test mode: generate a single embedding
        info!("Running in test mode...");

        let test_text = if args.len() > 2 {
            args[2].clone()
        } else {
            "This is a test sentence for embedding.".to_string()
        };

        match processor.embed_single(&test_text).await {
            Ok(embedding) => {
                println!("Embedding generated successfully!");
                println!("  Dimension: {}", embedding.len());
                println!("  First 5 values: {:?}", &embedding[..5.min(embedding.len())]);
            }
            Err(e) => {
                error!(error = %e, "Failed to generate embedding");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }

        return Ok(());
    }

    // Service mode: poll SQS queue
    info!("Embedding worker ready, starting queue polling...");

    // Initialize embedding queue
    let embedding_queue = match std::env::var("EMBEDDING_QUEUE_URL") {
        Ok(url) => {
            info!(url = %url, "Connecting to embedding queue...");
            let queue_config = QueueConfig {
                url,
                dlq_url: std::env::var("DLQ_URL").ok(),
                ..Default::default()
            };
            Queue::new(queue_config).await?
        }
        Err(_) => {
            warn!("EMBEDDING_QUEUE_URL not set, waiting for shutdown signal...");
            tokio::signal::ctrl_c().await?;
            info!("Embedding worker shutting down");
            return Ok(());
        }
    };

    // Circuit breaker state
    let mut consecutive_failures = 0;
    const MAX_FAILURES: u32 = 5;
    const CIRCUIT_BREAK_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

    // Start polling loop
    loop {
        // Circuit breaker check
        if consecutive_failures >= MAX_FAILURES {
            warn!(
                failures = consecutive_failures,
                "Circuit breaker open, pausing..."
            );
            tokio::time::sleep(CIRCUIT_BREAK_DURATION).await;
            consecutive_failures = 0;
            info!("Circuit breaker reset, resuming...");
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received");
                break;
            }
            result = embedding_queue.receive::<EmbeddingJob>() => {
                match result {
                    Ok(messages) => {
                        for (job, receipt_handle) in messages {
                            info!(
                                job_id = %job.job_id,
                                chunk_count = job.chunks.len(),
                                "Received embedding job"
                            );

                            match processor.process_job(job.clone()).await {
                                Ok(()) => {
                                    consecutive_failures = 0;
                                    // Delete message on success
                                    if let Err(e) = embedding_queue.delete(&receipt_handle).await {
                                        error!(error = %e, "Failed to delete message");
                                    }
                                }
                                Err(e) => {
                                    consecutive_failures += 1;
                                    error!(
                                        job_id = %job.job_id,
                                        error = %e,
                                        failures = consecutive_failures,
                                        "Failed to process embedding job"
                                    );
                                    // Message will be re-delivered or moved to DLQ
                                }
                            }
                        }
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        error!(error = %e, "Failed to receive messages from queue");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }

    info!("Embedding worker shutting down");
    Ok(())
}
