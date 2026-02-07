//! Tenant entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tenants")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    #[sea_orm(column_type = "Text", unique)]
    pub name: String,
    
    #[sea_orm(column_type = "Text")]
    pub api_key_hash: String,
    
    pub rate_limit_rps: i32,
    
    pub is_active: bool,
    
    pub created_at: DateTimeWithTimeZone,
    
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::paper::Entity")]
    Papers,
    
    #[sea_orm(has_many = "super::ingestion_job::Entity")]
    IngestionJobs,
    
    #[sea_orm(has_many = "super::session::Entity")]
    Sessions,
}

impl Related<super::paper::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Papers.def()
    }
}

impl Related<super::ingestion_job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::IngestionJobs.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
