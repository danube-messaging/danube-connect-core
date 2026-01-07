//! Configuration management for connectors.

use crate::{ConnectorError, ConnectorResult};
use danube_client::SubType;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Main configuration for connectors
///
/// Can be created via:
/// - `ConnectorConfig::from_env()` - load from environment variables
/// - `ConnectorConfig::from_file()` - load from TOML file
/// - Direct construction in code for full programmatic control
///
/// # Structure
/// - **Mandatory fields**: `danube_service_url`, `connector_name`
/// - **Optional fields**: `retry`, `processing`, `schemas` (with defaults)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// Danube broker service URL (mandatory, from DANUBE_SERVICE_URL env var)
    pub danube_service_url: String,

    /// Connector name (mandatory, from CONNECTOR_NAME env var, must be unique)
    pub connector_name: String,

    /// Retry settings (optional, from config file or defaults)
    #[serde(default)]
    pub retry: RetrySettings,

    /// Processing and runtime settings (optional, from config file or defaults)
    #[serde(default)]
    pub processing: ProcessingSettings,

    /// Schema mappings for topics (optional, for source connectors using schema registry)
    #[serde(default)]
    pub schemas: Vec<SchemaMapping>,
}

impl ConnectorConfig {
    /// Load mandatory configuration from environment variables
    ///
    /// Only reads mandatory fields:
    /// - `DANUBE_SERVICE_URL`: Danube broker URL (required)
    /// - `CONNECTOR_NAME`: Unique connector name (required)
    ///
    /// All retry and processing settings use defaults.
    /// To customize these, load from a config file or set them explicitly.
    pub fn from_env() -> ConnectorResult<Self> {
        let danube_service_url = env::var("DANUBE_SERVICE_URL")
            .map_err(|_| ConnectorError::config("DANUBE_SERVICE_URL is required"))?;

        let connector_name = env::var("CONNECTOR_NAME")
            .map_err(|_| ConnectorError::config("CONNECTOR_NAME is required"))?;

        Ok(Self {
            danube_service_url,
            connector_name,
            retry: RetrySettings::default(),
            processing: ProcessingSettings::default(),
            schemas: Vec::new(),
        })
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> ConnectorResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ConnectorError::config(format!("Failed to read config file {}: {}", path, e))
        })?;

        toml::from_str(&content).map_err(|e| {
            ConnectorError::config(format!("Failed to parse config file {}: {}", path, e))
        })
    }

    /// Apply environment variable overrides to mandatory fields only
    ///
    /// This only overrides `danube_service_url` and `connector_name`.
    /// Retry and processing settings should come from config files, not env vars.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = env::var("DANUBE_SERVICE_URL") {
            self.danube_service_url = val;
        }
        if let Ok(val) = env::var("CONNECTOR_NAME") {
            self.connector_name = val;
        }
    }

    /// Validate the configuration
    ///
    /// Called internally by the runtime. Users can also call this for early validation.
    pub(crate) fn validate(&self) -> ConnectorResult<()> {
        if self.danube_service_url.is_empty() {
            return Err(ConnectorError::config("danube_service_url cannot be empty"));
        }

        if self.connector_name.is_empty() {
            return Err(ConnectorError::config("connector_name cannot be empty"));
        }

        if self.retry.max_retries > 100 {
            return Err(ConnectorError::config("max_retries too high (max 100)"));
        }

        if self.processing.batch_size == 0 {
            return Err(ConnectorError::config("batch_size must be > 0"));
        }

        Ok(())
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "default-connector".to_string(),
            retry: RetrySettings::default(),
            processing: ProcessingSettings::default(),
            schemas: Vec::new(),
        }
    }
}

/// Retry configuration settings
///
/// Can be used via:
/// - TOML config file: `[retry]` section
/// - Direct construction in code for programmatic control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySettings {
    /// Maximum number of retries for failed operations
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Base backoff duration in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Maximum backoff duration in milliseconds
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

fn default_max_retries() -> u32 {
    3
}
fn default_retry_backoff_ms() -> u64 {
    1000
}
fn default_max_backoff_ms() -> u64 {
    30000
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_backoff_ms: 1000,
            max_backoff_ms: 30000,
        }
    }
}

/// Processing and runtime configuration settings
///
/// Can be used via:
/// - TOML config file: `[processing]` section
/// - Direct construction in code for programmatic control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingSettings {
    /// Batch size for batch processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Batch timeout in milliseconds
    #[serde(default = "default_batch_timeout_ms")]
    pub batch_timeout_ms: u64,

    /// Poll interval in milliseconds for source connectors
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,

    /// Metrics export port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_batch_size() -> usize {
    1000
}
fn default_batch_timeout_ms() -> u64 {
    1000
}
fn default_poll_interval_ms() -> u64 {
    100
}
fn default_metrics_port() -> u16 {
    9090
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            batch_timeout_ms: 1000,
            poll_interval_ms: 100,
            metrics_port: 9090,
            log_level: "info".to_string(),
        }
    }
}

/// Schema mapping configuration for topics
///
/// Maps a topic to its schema definition for source connectors using schema registry.
///
/// Can be used via:
/// - TOML config file: `[[schemas]]` array
/// - Direct construction in code for programmatic control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMapping {
    /// Danube topic name (format: /{namespace}/{topic_name})
    pub topic: String,

    /// Schema subject name in the registry
    pub subject: String,

    /// Schema type (e.g., "json_schema", "avro", "protobuf")
    pub schema_type: String,

    /// Path to schema definition file
    pub schema_file: PathBuf,

    /// Auto-register schema on startup if it doesn't exist
    #[serde(default = "default_auto_register")]
    pub auto_register: bool,

    /// Version strategy for this schema
    #[serde(default)]
    pub version_strategy: VersionStrategy,
}

fn default_auto_register() -> bool {
    true
}

/// Subscription type for configuration
///
/// **Mandatory public API** - required for `ConsumerConfig`.
///
/// Mirrors `SubType` from danube-client but with Serialize/Deserialize for config files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionType {
    Exclusive,
    Shared,
    FailOver,
}

impl From<SubscriptionType> for SubType {
    fn from(st: SubscriptionType) -> Self {
        match st {
            SubscriptionType::Exclusive => SubType::Exclusive,
            SubscriptionType::Shared => SubType::Shared,
            SubscriptionType::FailOver => SubType::FailOver,
        }
    }
}

/// Configuration for a Danube consumer
///
/// **Mandatory public API** - required by `SinkConnector::consumer_configs()` trait.
///
/// Specifies how to create a consumer for a specific topic, including subscription settings.
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    /// Danube topic to consume from (format: /{namespace}/{topic_name})
    pub topic: String,
    /// Consumer name (for identification)
    pub consumer_name: String,
    /// Subscription name (shared across consumer instances)
    pub subscription: String,
    /// Subscription type (Exclusive, Shared, FailOver)
    pub subscription_type: SubscriptionType,
    /// Optional: Expected schema subject for validation
    /// If set, runtime will validate that incoming messages match this schema
    pub expected_schema_subject: Option<String>,
}

/// Configuration for a Danube producer
///
/// **Mandatory public API** - required by `SourceConnector::producer_configs()` trait.
///
/// Specifies how to create a producer for a specific topic, including partitioning
/// and reliability settings.
///
/// Note: The `schema_config` field is internal and populated by the runtime from `SchemaMapping`.
/// Always use `ProducerConfig::new()` or set `schema_config` to `None` when constructing manually.
#[derive(Debug, Clone)]
pub struct ProducerConfig {
    /// Danube topic name (format: /{namespace}/{topic_name})
    pub topic: String,
    /// Number of partitions (0 = non-partitioned)
    pub partitions: usize,
    /// Use reliable dispatch (WAL + Cloud persistence)
    pub reliable_dispatch: bool,
    /// Internal: Schema configuration (populated by runtime from SchemaMapping)
    /// Users should not set this - always use None
    pub schema_config: Option<SchemaConfig>,
}

impl ProducerConfig {
    /// Create a new ProducerConfig
    pub fn new(topic: impl Into<String>, partitions: usize, reliable_dispatch: bool) -> Self {
        Self {
            topic: topic.into(),
            partitions,
            reliable_dispatch,
            schema_config: None,
        }
    }
}

/// Schema configuration for a topic
///
/// Can be used via:
/// - TOML config file: Use `SchemaMapping` in `[[schemas]]` (recommended)
/// - Direct construction in code for advanced programmatic control
///
/// Note: Usually populated automatically from `SchemaMapping` by the runtime.
///
/// The runtime converts `SchemaMapping` → `SchemaConfig` automatically when loading from files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaConfig {
    /// Schema subject name in the registry
    pub subject: String,

    /// Schema type (JsonSchema, Avro, Protobuf, etc.)
    pub schema_type: String,

    /// Path to schema definition file
    pub schema_file: PathBuf,

    /// Auto-register schema on startup if it doesn't exist
    #[serde(default = "default_schema_auto_register")]
    pub auto_register: bool,

    /// Version strategy for producers
    #[serde(default)]
    pub version_strategy: VersionStrategy,
}

fn default_schema_auto_register() -> bool {
    true
}

/// Strategy for selecting schema version
///
/// Used in `SchemaMapping` and `SchemaConfig` to control which schema version producers use.
///
/// Can be used via:
/// - TOML config file: `version_strategy` field in `[[schemas]]`
/// - Direct construction in code for programmatic control
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ConnectorConfig::default();
        assert_eq!(config.danube_service_url, "http://localhost:6650");
        assert_eq!(config.connector_name, "default-connector");
        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.processing.batch_size, 1000);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ConnectorConfig::default();
        assert!(config.validate().is_ok());

        config.danube_service_url = "".to_string();
        assert!(config.validate().is_err());

        config.danube_service_url = "http://localhost:6650".to_string();
        config.processing.batch_size = 0;
        assert!(config.validate().is_err());
    }
}
