//! Chunk entity with embedding versioning

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chunks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub paper_id: Uuid,
    
    pub chunk_index: i32,
    
    #[sea_orm(column_type = "Text")]
    pub content: String,
    
    /// pgvector embedding stored as text for SeaORM compatibility
    /// Actual vector operations done via raw SQL
    #[sea_orm(column_type = "Text", nullable)]
    pub embedding: Option<String>,
    
    /// Embedding model identifier for versioning
    #[sea_orm(column_type = "Text")]
    pub embedding_model: String,
    
    /// Embedding version number for model upgrades
    pub embedding_version: i32,
    
    /// Token count for context budgeting
    pub token_count: i32,
    
    /// Character offset in source document
    pub char_offset_start: Option<i32>,
    
    /// Character offset end in source document
    pub char_offset_end: Option<i32>,
    
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::paper::Entity",
        from = "Column::PaperId",
        to = "super::paper::Column::Id",
        on_delete = "Cascade"
    )]
    Paper,
}

impl Related<super::paper::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Paper.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Parse embedding from stored text format to Vec<f32>
    pub fn parse_embedding(&self) -> Option<Vec<f32>> {
        self.embedding.as_ref().and_then(|s| {
            // Format: "[1.0,2.0,3.0,...]"
            let inner = s.trim_start_matches('[').trim_end_matches(']');
            inner
                .split(',')
                .map(|v| v.trim().parse::<f32>().ok())
                .collect()
        })
    }
}
