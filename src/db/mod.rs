//! Database layer for PaperForge-rs
//!
//! Contains repository implementations for papers and chunks,
//! as well as SeaORM entity definitions.

pub mod models;
pub mod repository;

pub use models::{Paper, Chunk, PaperEntity, ChunkEntity};
pub use repository::Repository;
