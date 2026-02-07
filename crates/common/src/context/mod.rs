//! Context Engine Core Components
//!
//! The Context Engine is the intelligence layer that provides:
//! - Query understanding and expansion
//! - Multi-modal retrieval
//! - Context stitching
//! - Multi-hop reasoning
//! - LLM synthesis

mod query_parser;
mod context_stitcher;
mod reasoner;
mod synthesizer;

pub use query_parser::{QueryParser, QueryUnderstanding, Entity};
pub use context_stitcher::{ContextStitcher, ContextWindow, CrossReference};
pub use reasoner::{Reasoner, ReasoningChain, ReasoningHop};
pub use synthesizer::{Synthesizer, SynthesisOptions, SynthesizedAnswer, Citation};
