//! SeaORM entity models
//!
//! Database entities for PaperForge V2

mod paper;
mod chunk;
mod tenant;
mod ingestion_job;
mod citation;
mod session;

pub use paper::{
    Entity as PaperEntity,
    Model as Paper,
    ActiveModel as PaperActiveModel,
    Column as PaperColumn,
};

pub use chunk::{
    Entity as ChunkEntity,
    Model as Chunk,
    ActiveModel as ChunkActiveModel,
    Column as ChunkColumn,
};

pub use tenant::{
    Entity as TenantEntity,
    Model as Tenant,
    ActiveModel as TenantActiveModel,
    Column as TenantColumn,
};

pub use ingestion_job::{
    Entity as IngestionJobEntity,
    Model as IngestionJob,
    ActiveModel as IngestionJobActiveModel,
    Column as IngestionJobColumn,
    JobStatus,
};

pub use citation::{
    Entity as CitationEntity,
    Model as Citation,
    ActiveModel as CitationActiveModel,
    Column as CitationColumn,
};

pub use session::{
    Entity as SessionEntity,
    Model as Session,
    ActiveModel as SessionActiveModel,
    Column as SessionColumn,
};
