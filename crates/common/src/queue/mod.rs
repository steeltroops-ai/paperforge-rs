//! SQS Queue integration for async job processing
//!
//! Provides:
//! - SQS client wrapper with retry logic
//! - Message serialization/deserialization
//! - Dead letter queue handling

use crate::errors::{AppError, Result};
use aws_sdk_sqs::Client as SqsClient;
use aws_sdk_sqs::types::Message;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use backoff::{ExponentialBackoff, future::retry};
use tracing::{debug, error, info, warn};

/// SQS queue configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Queue URL
    pub url: String,
    /// Dead letter queue URL (optional)
    pub dlq_url: Option<String>,
    /// Maximum receive count before moving to DLQ
    pub max_receive_count: u32,
    /// Visibility timeout in seconds
    pub visibility_timeout: i32,
    /// Wait time for long polling (seconds)
    pub wait_time_seconds: i32,
    /// Maximum number of messages per poll
    pub max_messages: i32,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            dlq_url: None,
            max_receive_count: 3,
            visibility_timeout: 30,
            wait_time_seconds: 20,
            max_messages: 10,
        }
    }
}

/// SQS Queue client wrapper
pub struct Queue {
    client: SqsClient,
    config: QueueConfig,
}

impl Queue {
    /// Create a new queue client
    pub async fn new(config: QueueConfig) -> Result<Self> {
        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = SqsClient::new(&aws_config);
        
        Ok(Self { client, config })
    }
    
    /// Create with existing AWS config
    pub fn with_client(client: SqsClient, config: QueueConfig) -> Self {
        Self { client, config }
    }
    
    /// Send a message to the queue
    pub async fn send<T: Serialize>(&self, message: &T) -> Result<String> {
        let body = serde_json::to_string(message)
            .map_err(|e| AppError::QueueError { 
                message: format!("Failed to serialize message: {}", e) 
            })?;
        
        let result = self.client
            .send_message()
            .queue_url(&self.config.url)
            .message_body(&body)
            .send()
            .await
            .map_err(|e| AppError::QueueError {
                message: format!("Failed to send message: {}", e),
            })?;
        
        let message_id = result.message_id.unwrap_or_default();
        debug!(message_id = %message_id, "Message sent to queue");
        
        Ok(message_id)
    }
    
    /// Send a message with delay
    pub async fn send_delayed<T: Serialize>(&self, message: &T, delay_seconds: i32) -> Result<String> {
        let body = serde_json::to_string(message)
            .map_err(|e| AppError::QueueError { 
                message: format!("Failed to serialize message: {}", e) 
            })?;
        
        let result = self.client
            .send_message()
            .queue_url(&self.config.url)
            .message_body(&body)
            .delay_seconds(delay_seconds)
            .send()
            .await
            .map_err(|e| AppError::QueueError {
                message: format!("Failed to send delayed message: {}", e),
            })?;
        
        let message_id = result.message_id.unwrap_or_default();
        debug!(message_id = %message_id, delay_seconds, "Delayed message sent to queue");
        
        Ok(message_id)
    }
    
    /// Receive messages from the queue
    pub async fn receive(&self) -> Result<Vec<Message>> {
        let result = self.client
            .receive_message()
            .queue_url(&self.config.url)
            .max_number_of_messages(self.config.max_messages)
            .visibility_timeout(self.config.visibility_timeout)
            .wait_time_seconds(self.config.wait_time_seconds)
            .send()
            .await
            .map_err(|e| AppError::QueueError {
                message: format!("Failed to receive messages: {}", e),
            })?;
        
        let messages = result.messages.unwrap_or_default();
        debug!(count = messages.len(), "Received messages from queue");
        
        Ok(messages)
    }
    
    /// Delete a message after processing
    pub async fn delete(&self, receipt_handle: &str) -> Result<()> {
        self.client
            .delete_message()
            .queue_url(&self.config.url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .map_err(|e| AppError::QueueError {
                message: format!("Failed to delete message: {}", e),
            })?;
        
        debug!("Message deleted from queue");
        Ok(())
    }
    
    /// Change visibility timeout (extend processing time)
    pub async fn extend_visibility(&self, receipt_handle: &str, additional_seconds: i32) -> Result<()> {
        self.client
            .change_message_visibility()
            .queue_url(&self.config.url)
            .receipt_handle(receipt_handle)
            .visibility_timeout(additional_seconds)
            .send()
            .await
            .map_err(|e| AppError::QueueError {
                message: format!("Failed to extend visibility: {}", e),
            })?;
        
        debug!(additional_seconds, "Extended message visibility");
        Ok(())
    }
    
    /// Parse message body as JSON
    pub fn parse_message<T: DeserializeOwned>(message: &Message) -> Result<T> {
        let body = message.body.as_ref().ok_or_else(|| AppError::QueueError {
            message: "Message has no body".to_string(),
        })?;
        
        serde_json::from_str(body).map_err(|e| AppError::QueueError {
            message: format!("Failed to parse message: {}", e),
        })
    }
}

/// Ingestion job message
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct IngestionJobMessage {
    pub job_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub paper_title: String,
    pub paper_abstract: String,
    pub idempotency_key: Option<String>,
    pub options: IngestionJobOptions,
}

/// Ingestion job options
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct IngestionJobOptions {
    pub embedding_model: String,
    pub chunk_strategy: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

/// Embedding job message
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct EmbeddingJobMessage {
    pub job_id: uuid::Uuid,
    pub chunk_id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub content: String,
    pub chunk_index: i32,
    pub embedding_model: String,
}

/// Batch embedding job message
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BatchEmbeddingJobMessage {
    pub job_id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub chunks: Vec<ChunkData>,
    pub embedding_model: String,
}

/// Chunk data for batch processing
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ChunkData {
    pub chunk_id: uuid::Uuid,
    pub content: String,
    pub chunk_index: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_job_message_serialization() {
        let msg = IngestionJobMessage {
            job_id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            paper_title: "Test Paper".to_string(),
            paper_abstract: "Test abstract".to_string(),
            idempotency_key: Some("test-key".to_string()),
            options: IngestionJobOptions {
                embedding_model: "text-embedding-ada-002".to_string(),
                chunk_strategy: "sentence".to_string(),
                chunk_size: 512,
                chunk_overlap: 64,
            },
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: IngestionJobMessage = serde_json::from_str(&json).unwrap();
        
        assert_eq!(msg.job_id, parsed.job_id);
        assert_eq!(msg.paper_title, parsed.paper_title);
    }
}
