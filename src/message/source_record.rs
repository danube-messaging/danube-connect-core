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
    ///
    /// Use this for text-based data like log messages, plain text, or string values.
    ///
    /// # Example
    /// ```ignore
    /// let record = SourceRecord::from_string("/logs/application", "Server started successfully");
    /// let record = SourceRecord::from_string("/events/notifications", format!("User {} logged in", user_id));
    /// ```
    pub fn from_string(topic: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::new(topic, json!(payload.into()))
    }

    /// Create a SourceRecord from any JSON-serializable object
    ///
    /// Use this for structured data types that implement `Serialize`.
    /// The data will be converted to `serde_json::Value`.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Serialize)]
    /// struct OrderEvent {
    ///     order_id: String,
    ///     amount: f64,
    ///     currency: String,
    /// }
    ///
    /// let order = OrderEvent {
    ///     order_id: "ORD-12345".to_string(),
    ///     amount: 99.99,
    ///     currency: "USD".to_string(),
    /// };
    ///
    /// let record = SourceRecord::from_json("/orders/created", &order)?;
    /// ```
    pub fn from_json<T: Serialize>(topic: impl Into<String>, data: T) -> ConnectorResult<Self> {
        let value =
            serde_json::to_value(data).map_err(|e| ConnectorError::Serialization(e.to_string()))?;
        Ok(Self::new(topic, value))
    }

    /// Create a SourceRecord from a numeric value
    ///
    /// Supports integers and floats. The value will be stored as a JSON number.
    ///
    /// # Example
    /// ```ignore
    /// let record = SourceRecord::from_number("/metrics/counter", 42);
    /// let record = SourceRecord::from_number("/metrics/temperature", 23.5);
    /// ```
    pub fn from_number<T: Serialize>(topic: impl Into<String>, number: T) -> ConnectorResult<Self> {
        let value = serde_json::to_value(number)
            .map_err(|e| ConnectorError::Serialization(e.to_string()))?;
        
        // Ensure it's actually a number
        if !value.is_number() {
            return Err(ConnectorError::Serialization(
                "Value is not a number".to_string(),
            ));
        }
        
        Ok(Self::new(topic, value))
    }

    /// Create a SourceRecord from an Avro-compatible struct
    ///
    /// In Danube, Avro schemas use JSON serialization with schema validation.
    /// This is an alias for `from_json()` for clarity when working with Avro schemas.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Serialize)]
    /// struct UserEvent {
    ///     user_id: String,
    ///     action: String,
    ///     timestamp: i64,
    /// }
    ///
    /// let event = UserEvent { ... };
    /// let record = SourceRecord::from_avro("/events/users", &event)?;
    /// ```
    pub fn from_avro<T: Serialize>(topic: impl Into<String>, data: T) -> ConnectorResult<Self> {
        // Avro in Danube uses JSON serialization
        Self::from_json(topic, data)
    }

    /// Create a SourceRecord from binary data (base64-encoded)
    ///
    /// The bytes will be base64-encoded and stored as a JSON object.
    ///
    /// # Example
    /// ```ignore
    /// let binary_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    /// let record = SourceRecord::from_bytes("/binary/data", binary_data);
    /// ```
    pub fn from_bytes(topic: impl Into<String>, data: Vec<u8>) -> Self {
        let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
        Self::new(
            topic,
            json!({
                "data": base64_data,
                "size": data.len()
            }),
        )
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
                // Avro in Danube uses JSON serialization with schema validation
                serde_json::to_vec(&self.payload).map_err(|e| {
                    ConnectorError::Serialization(format!("Avro (JSON) serialization failed: {}", e))
                })
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

    #[test]
    fn test_source_record_from_number() {
        // Integer
        let record = SourceRecord::from_number("/metrics/counter", 42).unwrap();
        assert_eq!(record.payload, json!(42));
        assert_eq!(record.payload.as_i64().unwrap(), 42);

        // Float
        let record = SourceRecord::from_number("/metrics/temperature", 23.5).unwrap();
        assert_eq!(record.payload.as_f64().unwrap(), 23.5);

        // Negative number
        let record = SourceRecord::from_number("/metrics/balance", -100).unwrap();
        assert_eq!(record.payload.as_i64().unwrap(), -100);
    }

    #[test]
    fn test_source_record_from_avro() {
        #[derive(Serialize)]
        struct UserEvent {
            user_id: String,
            action: String,
            timestamp: i64,
        }

        let event = UserEvent {
            user_id: "user-123".to_string(),
            action: "login".to_string(),
            timestamp: 1234567890,
        };

        let record = SourceRecord::from_avro("/events/users", &event).unwrap();
        assert_eq!(record.payload["user_id"], "user-123");
        assert_eq!(record.payload["action"], "login");
        assert_eq!(record.payload["timestamp"], 1234567890);
    }

    #[test]
    fn test_source_record_from_bytes() {
        let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        let record = SourceRecord::from_bytes("/binary/data", data.clone());

        // Check the structure
        assert!(record.payload.is_object());
        assert!(record.payload["data"].is_string());
        assert_eq!(record.payload["size"], 5);

        // Verify base64 encoding
        let base64_data = record.payload["data"].as_str().unwrap();
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            base64_data,
        )
        .unwrap();
        assert_eq!(decoded, data);
    }
}
