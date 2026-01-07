//! Message transformation utilities.
//!
//! This module provides helper types and methods for transforming messages between
//! Danube's format and connector-specific formats.

use crate::{ConnectorError, ConnectorResult};

// Re-export SchemaInfo from danube-client (v0.6.1+)
pub use danube_client::SchemaInfo;
use danube_core::message::StreamMessage;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Record passed to sink connectors (from Danube → External System)
///
/// Sink connectors receive typed data as `serde_json::Value`, already deserialized
/// by the runtime based on the message's schema.
#[derive(Debug, Clone)]
pub struct SinkRecord {
    /// The actual message payload (typed data, already deserialized)
    pub payload: Value,
    /// User-defined attributes/properties from producer
    pub attributes: HashMap<String, String>,
    /// Danube metadata for observability and debugging
    pub danube_metadata: DanubeMetadata,
    /// Topic partition (if topic is partitioned)
    pub partition: Option<String>,
    /// Schema information (if message has schema)
    pub schema_info: Option<SchemaInfo>,
}

/// Danube-specific metadata for observability and debugging
#[derive(Debug, Clone)]
pub struct DanubeMetadata {
    /// Topic name
    pub topic: String,
    /// Message offset within topic
    pub offset: u64,
    /// Publish timestamp (microseconds since epoch)
    pub publish_time: u64,
    /// Formatted message ID for logging/debugging
    pub message_id: String,
    /// Producer name (for debugging)
    pub producer_name: String,
}

impl SinkRecord {
    /// Convert Danube StreamMessage to SinkRecord
    /// Note: This is used internally. Runtime will deserialize payload based on schema.
    pub fn from_stream_message(message: StreamMessage, partition: Option<String>) -> Self {
        let message_id = format!(
            "topic:{}/producer:{}/offset:{}",
            message.msg_id.topic_name, message.msg_id.producer_id, message.msg_id.topic_offset
        );

        // For backward compatibility during transition, try to deserialize payload
        let payload = serde_json::from_slice(&message.payload)
            .unwrap_or_else(|_| json!({ "raw": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload) }));

        SinkRecord {
            payload,
            attributes: message.attributes,
            danube_metadata: DanubeMetadata {
                topic: message.msg_id.topic_name.clone(),
                offset: message.msg_id.topic_offset,
                publish_time: message.publish_time,
                message_id,
                producer_name: message.producer_name,
            },
            partition,
            schema_info: None, // Will be set by runtime when using schema-aware deserialization
        }
    }

    /// Get the payload as a reference
    pub fn payload(&self) -> &Value {
        &self.payload
    }

    /// Deserialize payload to a specific type
    ///
    /// The payload is already deserialized by the runtime based on the message's schema.
    /// This method converts the `Value` to your connector's data type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct VectorData {
    ///     vector: Vec<f32>,
    ///     metadata: HashMap<String, String>,
    /// }
    ///
    /// let data: VectorData = record.as_type()?;
    /// ```
    pub fn as_type<T: DeserializeOwned>(&self) -> ConnectorResult<T> {
        serde_json::from_value(self.payload.clone()).map_err(|e| ConnectorError::InvalidData {
            message: format!("Failed to deserialize to target type: {}", e),
            payload: serde_json::to_vec(&self.payload).unwrap_or_default(),
        })
    }

    /// Get schema information for this message
    pub fn schema(&self) -> Option<&SchemaInfo> {
        self.schema_info.as_ref()
    }

    /// Access message attributes (user-defined properties)
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    /// Get a specific attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    /// Check if an attribute exists
    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get the topic name
    pub fn topic(&self) -> &str {
        &self.danube_metadata.topic
    }

    /// Get the topic offset
    pub fn offset(&self) -> u64 {
        self.danube_metadata.offset
    }

    /// Get the publish timestamp (microseconds since epoch)
    pub fn publish_time(&self) -> u64 {
        self.danube_metadata.publish_time
    }

    /// Get the producer name
    pub fn producer_name(&self) -> &str {
        &self.danube_metadata.producer_name
    }

    /// Get a formatted message ID string for logging
    pub fn message_id(&self) -> &str {
        &self.danube_metadata.message_id
    }
}

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
    use danube_core::message::MessageID;
    use serde::{Deserialize, Serialize};

    fn create_test_message() -> StreamMessage {
        StreamMessage {
            request_id: 1,
            msg_id: MessageID {
                producer_id: 100,
                topic_name: "/default/test".to_string(),
                broker_addr: "localhost:6650".to_string(),
                topic_offset: 42,
            },
            payload: serde_json::to_vec(&json!("test payload")).unwrap(), // JSON string
            publish_time: 1234567890,
            producer_name: "test-producer".to_string(),
            subscription_name: Some("test-sub".to_string()),
            attributes: HashMap::new(),
            schema_id: None,      // NEW in v0.6.1 - schema registry support
            schema_version: None, // NEW in v0.6.1 - schema registry support
        }
    }

    #[test]
    fn test_sink_record_basic() {
        let message = create_test_message();
        let record = SinkRecord::from_stream_message(message, None);

        // Payload is now typed data (Value)
        assert_eq!(record.payload().as_str().unwrap(), "test payload");
        assert_eq!(record.topic(), "/default/test");
        assert_eq!(record.offset(), 42);
        assert_eq!(record.producer_name(), "test-producer");
    }

    #[test]
    fn test_sink_record_as_type() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let mut message = create_test_message();
        message.payload = serde_json::to_vec(&data).unwrap();

        let record = SinkRecord::from_stream_message(message, None);
        let decoded: TestData = record.as_type().unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn test_sink_record_payload_access() {
        let mut message = create_test_message();
        let data = json!({"name": "test", "value": 42});
        message.payload = serde_json::to_vec(&data).unwrap();

        let record = SinkRecord::from_stream_message(message, None);

        // Direct access to payload
        assert_eq!(record.payload()["name"], "test");
        assert_eq!(record.payload()["value"], 42);
    }

    #[test]
    fn test_sink_record_schema_info() {
        let message = create_test_message();
        let record = SinkRecord::from_stream_message(message, None);

        // No schema info by default (runtime will set it in Phase 2)
        assert!(record.schema().is_none());
    }

    #[test]
    fn test_sink_record_attributes() {
        let mut message = create_test_message();
        message
            .attributes
            .insert("key1".to_string(), "value1".to_string());

        let record = SinkRecord::from_stream_message(message, None);

        assert_eq!(record.get_attribute("key1"), Some("value1"));
        assert_eq!(record.get_attribute("key2"), None);
        assert!(record.has_attribute("key1"));
        assert!(!record.has_attribute("key2"));
    }

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
