//! Configuration management for connectors.

use crate::{ConnectorError, ConnectorResult, VersionStrategy};
use danube_client::SubType;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Main configuration for connectors
///
/// # Structure
/// - **Mandatory fields** (from environment): `danube_service_url`, `connector_name`
/// - **Optional fields** (from config file or defaults): `retry`, `processing`, `schemas`
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
    pub fn validate(&self) -> ConnectorResult<()> {
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
/// Maps a topic to its schema definition for source connectors using schema registry
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

/// Subscription type for configuration (mirrors SubType but with Serialize/Deserialize)
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
