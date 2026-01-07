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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod context;
mod sink_runtime;
mod source_runtime;

pub use context::{CacheStats, ConnectorContext};
pub use sink_runtime::{ConsumerConfig, SinkRuntime};
pub use source_runtime::{ProducerConfig, SourceRuntime};

/// Schema configuration for a topic
///
/// Defines how messages for a topic should be serialized/deserialized
/// using the schema registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaConfig {
    /// Schema subject name in the registry
    pub subject: String,
    
    /// Schema type (JsonSchema, Avro, Protobuf, etc.)
    /// TODO Phase 2: Change to use actual SchemaType from danube-client v0.6.1+
    pub schema_type: String,
    
    /// Path to schema definition file
    pub schema_file: PathBuf,
    
    /// Auto-register schema on startup if it doesn't exist
    #[serde(default = "default_auto_register")]
    pub auto_register: bool,
    
    /// Version strategy for producers
    #[serde(default)]
    pub version_strategy: VersionStrategy,
}

fn default_auto_register() -> bool {
    true
}

/// Strategy for selecting schema version
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionStrategy {
    /// Use the latest schema version (default)
    #[default]
    Latest,
    
    /// Pin to a specific schema version
    Pinned(u32),
    
    /// Use minimum version or newer
    Minimum(u32),
}
