//! Database models for PaperForge-rs
//! 
//! Uses SeaORM entities with separate modules for papers and chunks.
//! Note: pgvector embedding type is handled via raw SQL since SeaORM 
//! doesn't have native support.

pub mod paper;
pub mod chunk;

pub use paper::Entity as PaperEntity;
pub use paper::Model as Paper;
pub use paper::ActiveModel as PaperActiveModel;
pub use paper::Column as PaperColumn;

pub use chunk::Entity as ChunkEntity;
pub use chunk::Model as Chunk;
pub use chunk::ActiveModel as ChunkActiveModel;
pub use chunk::Column as ChunkColumn;
