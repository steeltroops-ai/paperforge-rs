//! Chunk entity

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
    
    /// pgvector type - handled via raw SQL
    /// SeaORM doesn't natively support pgvector, so we use optional String
    /// and raw SQL for actual vector operations
    #[sea_orm(column_type = "Text", nullable)]
    pub embedding: Option<String>,
    
    /// Embedding model used (for versioning)
    #[sea_orm(column_type = "Text")]
    pub embedding_model: String,
    
    /// Embedding version (for versioning)
    pub embedding_version: i32,
    
    pub token_count: i32,
    
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::paper::Entity",
        from = "Column::PaperId",
        to = "super::paper::Column::Id",
        on_update = "NoAction",
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
