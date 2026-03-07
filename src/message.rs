//! Message transformation utilities.
//!
//! This module provides helper types and methods for transforming messages between
//! Danube's format and connector-specific formats.

mod context;
mod sink_record;
mod source_record;

// Re-export message types
pub use context::{RecordContext, RoutingContext};
pub use sink_record::{DanubeMetadata, SinkRecord};
pub use source_record::SourceRecord;
