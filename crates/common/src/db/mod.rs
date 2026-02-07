//! Database layer for PaperForge
//!
//! Provides:
//! - SeaORM entity models
//! - Repository pattern for data access
//! - Connection pool management
//! - Query helpers

pub mod models;
mod repository;

pub use repository::{ChunkResult, Repository};

use crate::config::DatabaseConfig;
use crate::errors::{AppError, Result};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::info;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DbPool {
    /// Primary connection (for writes)
    pub primary: DatabaseConnection,
    
    /// Read replica connection (optional)
    pub replica: Option<DatabaseConnection>,
}

impl DbPool {
    /// Create a new database pool from configuration
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to primary database...");
        
        let mut primary_opts = ConnectOptions::new(&config.url);
        primary_opts
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .sqlx_logging(true);
        
        let primary = Database::connect(primary_opts)
            .await
            .map_err(|e| AppError::DatabaseConnection { 
                message: format!("Failed to connect to primary: {}", e) 
            })?;
        
        // Connect to replica if configured
        let replica = if let Some(ref read_url) = config.read_url {
            info!("Connecting to read replica...");
            
            let mut replica_opts = ConnectOptions::new(read_url);
            replica_opts
                .max_connections(config.max_connections)
                .min_connections(config.min_connections)
                .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
                .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
                .sqlx_logging(true);
            
            let replica_conn = Database::connect(replica_opts)
                .await
                .map_err(|e| AppError::DatabaseConnection { 
                    message: format!("Failed to connect to replica: {}", e) 
                })?;
            
            Some(replica_conn)
        } else {
            None
        };
        
        info!("Database connections established");
        
        Ok(Self { primary, replica })
    }
    
    /// Get the connection for reads (replica if available, otherwise primary)
    pub fn read(&self) -> &DatabaseConnection {
        self.replica.as_ref().unwrap_or(&self.primary)
    }
    
    /// Get the connection for writes (always primary)
    pub fn write(&self) -> &DatabaseConnection {
        &self.primary
    }
    
    /// Ping the database to check connectivity
    pub async fn ping(&self) -> Result<()> {
        use sea_orm::ConnectionTrait;
        
        self.primary
            .execute_unprepared("SELECT 1")
            .await
            .map_err(|e| AppError::DatabaseConnection {
                message: format!("Primary ping failed: {}", e),
            })?;
        
        if let Some(ref replica) = self.replica {
            replica
                .execute_unprepared("SELECT 1")
                .await
                .map_err(|e| AppError::DatabaseConnection {
                    message: format!("Replica ping failed: {}", e),
                })?;
        }
        
        Ok(())
    }
}
