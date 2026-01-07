//! SourceRecord - messages from external systems to Danube

use crate::{ConnectorError, ConnectorResult};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Record passed from source connectors (External System → Danube)
///
/// Source connectors emit typed data as `serde_json::Value`. The runtime handles
/// schema-based serialization before sending to Danube.
#[derive(Debug, Clone, Serialize)]
pub struct SourceRecord {
    /// The topic to publish to
    pub topic: String,
    /// The message payload (typed data, not bytes)
    pub payload: Value,
    /// Optional message attributes/headers
    pub attributes: HashMap<String, String>,
    /// Optional routing key for partitioned topics (will be used when Danube supports it)
    pub key: Option<String>,
}

impl SourceRecord {
    /// Create a new SourceRecord with typed payload
    pub fn new(topic: impl Into<String>, payload: Value) -> Self {
        Self {
            topic: topic.into(),
            payload,
            attributes: HashMap::new(),
            key: None,
        }
    }

    /// Create a SourceRecord from a string payload
    pub fn from_string(topic: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::new(topic, json!(payload.into()))
    }

    /// Create a SourceRecord from any JSON-serializable object
    pub fn from_json<T: Serialize>(topic: impl Into<String>, data: T) -> ConnectorResult<Self> {
        let value =
            serde_json::to_value(data).map_err(|e| ConnectorError::Serialization(e.to_string()))?;
        Ok(Self::new(topic, value))
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Add multiple attributes
    pub fn with_attributes(mut self, attrs: HashMap<String, String>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    /// Set the routing key for partitioned topics
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Get the payload as a reference
    pub fn payload(&self) -> &Value {
        &self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn test_source_record_basic() {
        let record = SourceRecord::new("/default/events", json!("test"));

        assert_eq!(record.topic, "/default/events");
        assert_eq!(record.payload, json!("test"));
        assert!(record.attributes.is_empty());
        assert!(record.key.is_none());
    }

    #[test]
    fn test_source_record_from_string() {
        let record = SourceRecord::from_string("/default/events", "test message");

        assert_eq!(record.payload, json!("test message"));
        assert_eq!(record.payload.as_str().unwrap(), "test message");
    }

    #[test]
    fn test_source_record_from_json() {
        #[derive(Serialize)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let record = SourceRecord::from_json("/default/events", data).unwrap();

        assert_eq!(record.payload["name"], "test");
        assert_eq!(record.payload["value"], 42);
    }

    #[test]
    fn test_source_record_builder() {
        let record = SourceRecord::new("/default/events", json!("test"))
            .with_attribute("source", "test-connector")
            .with_attribute("version", "1.0")
            .with_key("user-123");

        assert_eq!(
            record.attributes.get("source"),
            Some(&"test-connector".to_string())
        );
        assert_eq!(record.attributes.get("version"), Some(&"1.0".to_string()));
        assert_eq!(record.key, Some("user-123".to_string()));
    }
}
