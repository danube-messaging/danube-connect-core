//! SourceRecord - messages from external systems to Danube

use crate::{ConnectorError, ConnectorResult};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::warn;

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

    /// Serialize payload to bytes based on schema type
    ///
    /// Converts serde_json::Value to bytes according to the schema type.
    /// The actual schema validation happens in the broker.
    pub(crate) fn serialize_with_schema(&self, schema_type: &str) -> ConnectorResult<Vec<u8>> {
        match schema_type.to_lowercase().as_str() {
            "json_schema" | "json" => {
                // JSON Schema - serialize as JSON
                serde_json::to_vec(&self.payload).map_err(|e| {
                    ConnectorError::Serialization(format!("JSON serialization failed: {}", e))
                })
            }
            "string" => {
                // String type - convert to UTF-8 bytes
                if let Some(s) = self.payload.as_str() {
                    Ok(s.as_bytes().to_vec())
                } else {
                    // If not a string, serialize as JSON string
                    Ok(self.payload.to_string().into_bytes())
                }
            }
            "number" => {
                // Number - serialize as JSON number
                serde_json::to_vec(&self.payload).map_err(|e| {
                    ConnectorError::Serialization(format!("Number serialization failed: {}", e))
                })
            }
            "bytes" => {
                // Bytes - try to extract from base64 string or object
                if let Some(s) = self.payload.as_str() {
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).map_err(
                        |e| ConnectorError::Serialization(format!("Invalid base64: {}", e)),
                    )
                } else if let Some(obj) = self.payload.as_object() {
                    if let Some(data) = obj.get("data").and_then(|v| v.as_str()) {
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                            .map_err(|e| {
                                ConnectorError::Serialization(format!("Invalid base64: {}", e))
                            })
                    } else {
                        Err(ConnectorError::Serialization(
                            "Expected 'data' field with base64 string".to_string(),
                        ))
                    }
                } else {
                    Err(ConnectorError::Serialization(
                        "Cannot convert to bytes".to_string(),
                    ))
                }
            }
            "avro" => {
                // TODO: Implement Avro serialization
                Err(ConnectorError::config(
                    "Avro serialization not yet implemented",
                ))
            }
            "protobuf" => {
                // TODO: Implement Protobuf serialization
                Err(ConnectorError::config(
                    "Protobuf serialization not yet implemented",
                ))
            }
            _ => {
                // Unknown type - default to JSON
                warn!(
                    "Unknown schema type '{}', defaulting to JSON serialization",
                    schema_type
                );
                serde_json::to_vec(&self.payload).map_err(|e| {
                    ConnectorError::Serialization(format!("JSON serialization failed: {}", e))
                })
            }
        }
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
