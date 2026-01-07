//! Message transformation utilities.
//!
//! This module provides helper types and methods for transforming messages between
//! Danube's format and connector-specific formats.

mod sink_record;
mod source_record;

// Re-export message types
pub use sink_record::{DanubeMetadata, SinkRecord};
pub use source_record::SourceRecord;
