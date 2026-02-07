//! Session entity for context engine

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub tenant_id: Uuid,
    
    /// Session state as JSONB for flexibility
    #[sea_orm(column_type = "JsonBinary")]
    pub state: serde_json::Value,
    
    pub created_at: DateTimeWithTimeZone,
    
    pub last_active_at: DateTimeWithTimeZone,
    
    pub expires_at: DateTimeWithTimeZone,
}

impl Model {
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        use chrono::Utc;
        self.expires_at < Utc::now().into()
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
}

impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
