//! Paper entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "papers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    #[sea_orm(column_type = "Text")]
    pub title: String,
    
    #[sea_orm(column_type = "Text")]
    pub abstract_text: String,
    
    pub published_at: Option<DateTimeWithTimeZone>,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub source: Option<String>,
    
    /// Idempotency key for deduplication
    /// SHA256 hash of title + abstract or client-provided key
    #[sea_orm(column_type = "Text", nullable, unique)]
    pub idempotency_key: Option<String>,
    
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::chunk::Entity")]
    Chunks,
}

impl Related<super::chunk::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chunks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
