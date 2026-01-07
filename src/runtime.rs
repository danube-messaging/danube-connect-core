//! Runtime for managing connector lifecycle.
//!
//! This module provides runtime implementations for both sink and source connectors:
//! - `SinkRuntime`: Handles Danube → External System (consuming from Danube)
//! - `SourceRuntime`: Handles External System → Danube (producing to Danube)
//!
//! The runtimes handle:
//! - Connector initialization
//! - Danube client setup and connection management
//! - Message processing loops
//! - Retry logic
//! - Health monitoring
//! - Graceful shutdown

mod context;
mod sink_runtime;
mod source_runtime;

// Re-export internal types for use within the crate
pub(crate) use context::ConnectorContext;

// Re-export public API types
pub use sink_runtime::SinkRuntime;
pub use source_runtime::SourceRuntime;
