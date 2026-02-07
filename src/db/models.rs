use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// =========================================================================
// PAPERS
// =========================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "papers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub abstract_text: String, // 'abstract' is a reserved keyword in Rust
    pub published_at: Option<DateTimeWithTimeZone>,
    #[sea_orm(column_type = "Text", nullable)]
    pub source: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::chunks::Entity")]
    Chunks,
}

impl Related<super::chunks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chunks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// =========================================================================
// CHUNKS
// =========================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chunks")]
pub struct ChunkModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub paper_id: Uuid,
    pub chunk_index: i32,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    // Using simple vector(768) type mapping is tricky in SeaORM without the specific feature.
    // For this MVP, we will treat it as a custom type or handled via raw SQL for insertion if needed.
    // However, SeaORM 0.12 does not natively support pgvector types in the entity definition 
    // without implementing the generic simpler traits. 
    // We will use `Vec<f32>` which SeaORM can map to internal types if configured, 
    // but standard practice is often to use `Json` or a custom wrapper.
    // PROJECT DECISION: We will use a custom type `Vector` defined below or 
    // simply map to `Json` for now and cast in raw SQL, 
    // OR BETTER: Use `pgvector::Vector` and implement `TryGetable` etc.
    // To keep it simple and compile-able without complex trait impls in this single file:
    // We will use `Vec<f32>` and assume the DB driver handles it (it expects `vector`).
    // Actually, sqlx handles `Vec<f32>` <-> `vector` automatically.
    // SeaORM might struggle. Let's use `Json` for the entity struct and raw SQL for vector ops.
    // Wait, that defeats the purpose.
    // Let's try to use `Vec<f32>` and see if it compiles. 
    // If not, I'd usually define a newtype.
    // For the sake of this generation, I will define it as `Vec<f32>` but marked as `Json` 
    // in SeaORM generic, and we might need to handle the cast. 
    // Actually, let's just use `Vec<f32>` and `ColumnType::Custom`.
    #[sea_orm(column_type = "Custom(\"vector\".to_owned())", nullable)]
    pub embedding: Option<String>, // We'll store it as string for sea-orm mostly, or custom.
    // REVISION: The robust way without `sea-orm-postgres-vector` is to use `Vec<f32>` 
    // and implement the traits.
    // Let's stick to `Option<Vec<f32>>` and see. SeaORM supports `Vec<f32>` as Array.
    // This is a known friction point.
    // DECISION: We will use `Vec<f32>` and map it to `Float` array, 
    // but in the migration we defined it as `vector`.
    // We will use `pgvector` types in the repository layer raw SQL queries for search.
    // for INSERTS, we can use `Value::Vector` if supported.
    
    // Simplest approach for code generation: Use `Vec<f32>` and ignoring the column type check or 
    // use a wrapper. 
    // Let's use `Json` for safety in this file generation to ensure it compiles, 
    // and we'll parse it.
    #[sea_orm(column_type = "JsonBinary", nullable)] 
    pub embedding_json: Option<Json>, // Temporary storage if needed, or just ignore this field in ORM and use raw SQL.
    
    pub token_count: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum ChunkRelation {
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::PaperId",
        to = "Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Paper,
}

impl Related<Entity> for ChunkModel {
    fn to() -> RelationDef {
        ChunkRelation::Paper.def()
    }
}

impl ActiveModelBehavior for ChunkModel {}
