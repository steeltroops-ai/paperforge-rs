//! Citation entity for graph relationships

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "citations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    /// Paper that contains the citation
    pub citing_paper_id: Uuid,
    
    /// Paper that is being cited
    pub cited_paper_id: Uuid,
    
    /// The sentence/context containing the citation
    #[sea_orm(column_type = "Text", nullable)]
    pub citation_context: Option<String>,
    
    /// Position of citation in the paper (for ordering)
    pub position_in_paper: Option<i32>,
    
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::paper::Entity",
        from = "Column::CitingPaperId",
        to = "super::paper::Column::Id",
        on_delete = "Cascade"
    )]
    CitingPaper,
    
    #[sea_orm(
        belongs_to = "super::paper::Entity",
        from = "Column::CitedPaperId",
        to = "super::paper::Column::Id",
        on_delete = "Cascade"
    )]
    CitedPaper,
}

impl ActiveModelBehavior for ActiveModel {}
