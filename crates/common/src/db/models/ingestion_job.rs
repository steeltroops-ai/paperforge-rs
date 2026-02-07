//! Ingestion job entity for async processing

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Job status enum
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Chunking,
    Embedding,
    Indexing,
    Completed,
    Failed,
}

impl From<String> for JobStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => JobStatus::Pending,
            "chunking" => JobStatus::Chunking,
            "embedding" => JobStatus::Embedding,
            "indexing" => JobStatus::Indexing,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            _ => JobStatus::Pending,
        }
    }
}

impl From<JobStatus> for String {
    fn from(status: JobStatus) -> Self {
        match status {
            JobStatus::Pending => "pending".to_string(),
            JobStatus::Chunking => "chunking".to_string(),
            JobStatus::Embedding => "embedding".to_string(),
            JobStatus::Indexing => "indexing".to_string(),
            JobStatus::Completed => "completed".to_string(),
            JobStatus::Failed => "failed".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ingestion_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub tenant_id: Uuid,
    
    pub paper_id: Option<Uuid>,
    
    #[sea_orm(column_type = "Text")]
    pub status: String,
    
    pub chunks_total: i32,
    
    pub chunks_processed: i32,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub idempotency_key: Option<String>,
    
    pub attempt_count: i32,
    
    pub next_retry_at: Option<DateTimeWithTimeZone>,
    
    pub created_at: DateTimeWithTimeZone,
    
    pub started_at: Option<DateTimeWithTimeZone>,
    
    pub completed_at: Option<DateTimeWithTimeZone>,
}

impl Model {
    /// Get the job status as an enum
    pub fn job_status(&self) -> JobStatus {
        JobStatus::from(self.status.clone())
    }
    
    /// Check if the job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self.job_status(), JobStatus::Completed | JobStatus::Failed)
    }
    
    /// Calculate progress percentage
    pub fn progress_percent(&self) -> f64 {
        if self.chunks_total == 0 {
            0.0
        } else {
            (self.chunks_processed as f64 / self.chunks_total as f64) * 100.0
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    #[sea_orm(
        belongs_to = "super::paper::Entity",
        from = "Column::PaperId",
        to = "super::paper::Column::Id"
    )]
    Paper,
}

impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl Related<super::paper::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Paper.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
