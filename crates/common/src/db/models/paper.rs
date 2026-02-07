//! Paper entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "papers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub tenant_id: Uuid,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub external_id: Option<String>,
    
    #[sea_orm(column_type = "Text")]
    pub title: String,
    
    #[sea_orm(column_type = "Text")]
    pub abstract_text: String,
    
    pub published_at: Option<DateTimeWithTimeZone>,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub source: Option<String>,
    
    /// Extensible metadata as JSONB
    #[sea_orm(column_type = "JsonBinary")]
    pub metadata: serde_json::Value,
    
    /// Idempotency key for deduplication
    #[sea_orm(column_type = "Text", nullable)]
    pub idempotency_key: Option<String>,
    
    pub created_at: DateTimeWithTimeZone,
    
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    #[sea_orm(has_many = "super::chunk::Entity")]
    Chunks,
    
    #[sea_orm(has_many = "super::citation::Entity", on_delete = "Cascade")]
    CitationsFrom,
}

impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl Related<super::chunk::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chunks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
