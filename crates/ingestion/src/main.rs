//! PaperForge Ingestion Service
//!
//! Processes ingestion jobs from SQS queue:
//! 1. Receives job message
//! 2. Extracts text from PDF
//! 3. Chunks the paper text
//! 4. Sends chunks to embedding queue
//! 5. Updates job status

mod chunker;
mod errors;
mod pdf;
mod processor;

use crate::chunker::ChunkingConfig;
use crate::processor::{IngestionJobMessage, IngestionProcessor};
use paperforge_common::{
    config::AppConfig,
    db::DbPool,
    queue::{Queue, QueueConfig},
    VERSION,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn, Level};
use uuid::Uuid;

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
    let db = DbPool::new(&config.database).await?;

    // Initialize embedding queue (optional - may not be available locally)
    let embedding_queue = match std::env::var("EMBEDDING_QUEUE_URL") {
        Ok(url) => {
            info!(url = %url, "Connecting to embedding queue...");
            let queue_config = QueueConfig {
                url,
                dlq_url: std::env::var("DLQ_URL").ok(),
                ..Default::default()
            };
            match Queue::new(queue_config).await {
                Ok(queue) => Some(Arc::new(queue)),
                Err(e) => {
                    warn!(error = %e, "Failed to connect to embedding queue, running in standalone mode");
                    None
                }
            }
        }
        Err(_) => {
            warn!("EMBEDDING_QUEUE_URL not set, running in standalone mode");
            None
        }
    };

    // Initialize processor
    let processor = IngestionProcessor::new(
        db.clone(),
        embedding_queue.clone(),
        ChunkingConfig::default(),
        config.embedding.model.clone(),
    );

    // Check for command line arguments for local testing
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // CLI mode: process local files
        let command = &args[1];

        match command.as_str() {
            "process-file" => {
                if args.len() < 3 {
                    eprintln!("Usage: ingestion process-file <path-to-pdf>");
                    std::process::exit(1);
                }
                let path = PathBuf::from(&args[2]);
                let tenant_id = Uuid::new_v4(); // Use random tenant for testing

                info!(path = %path.display(), "Processing single PDF file");

                match processor.process_local_pdf(&path, tenant_id, None).await {
                    Ok((job_id, paper_id, chunks)) => {
                        info!(
                            job_id = %job_id,
                            paper_id = %paper_id,
                            chunk_count = chunks.len(),
                            "PDF processed successfully"
                        );
                        println!("Success!");
                        println!("  Job ID:      {}", job_id);
                        println!("  Paper ID:    {}", paper_id);
                        println!("  Chunks:      {}", chunks.len());
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to process PDF");
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "process-dir" => {
                if args.len() < 3 {
                    eprintln!("Usage: ingestion process-dir <path-to-directory>");
                    std::process::exit(1);
                }
                let path = PathBuf::from(&args[2]);
                let tenant_id = Uuid::new_v4();

                info!(path = %path.display(), "Processing directory of PDFs");

                match processor.process_directory(&path, tenant_id).await {
                    Ok(results) => {
                        println!("Processed {} PDFs:", results.len());
                        for (job_id, paper_id, chunk_count) in results {
                            println!(
                                "  Job: {} | Paper: {} | Chunks: {}",
                                job_id, paper_id, chunk_count
                            );
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to process directory");
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                eprintln!("Unknown command: {}", command);
                eprintln!("Available commands:");
                eprintln!("  process-file <path>  - Process a single PDF file");
                eprintln!("  process-dir <path>   - Process all PDFs in a directory");
                std::process::exit(1);
            }
        }

        return Ok(());
    }

    // Service mode: poll SQS queue
    info!("Ingestion service ready, starting queue polling...");

    // Initialize ingestion queue
    let ingestion_queue = match std::env::var("INGESTION_QUEUE_URL") {
        Ok(url) => {
            let queue_config = QueueConfig {
                url,
                dlq_url: std::env::var("DLQ_URL").ok(),
                ..Default::default()
            };
            Queue::new(queue_config).await?
        }
        Err(_) => {
            warn!("INGESTION_QUEUE_URL not set, waiting for shutdown signal...");
            tokio::signal::ctrl_c().await?;
            info!("Ingestion service shutting down");
            return Ok(());
        }
    };

    // Start polling loop
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received");
                break;
            }
            result = ingestion_queue.receive::<IngestionJobMessage>() => {
                match result {
                    Ok(messages) => {
                        for (message, receipt_handle) in messages {
                            info!(job_id = %message.job_id, "Received ingestion job");

                            match processor.process_job(message.clone()).await {
                                Ok(()) => {
                                    // Delete message on success
                                    if let Err(e) = ingestion_queue.delete(&receipt_handle).await {
                                        error!(error = %e, "Failed to delete message");
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        job_id = %message.job_id,
                                        error = %e,
                                        "Failed to process ingestion job"
                                    );
                                    // Message will be re-delivered or moved to DLQ
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to receive messages from queue");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }

    info!("Ingestion service shutting down");
    Ok(())
}
