//! SinkRecord - messages from Danube to external systems

use crate::runtime::ConnectorContext;
use crate::{ConnectorError, ConnectorResult};
use danube_client::SchemaInfo;
use danube_core::message::StreamMessage;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Record passed to sink connectors (from Danube → External System)
///
/// Sink connectors receive typed data as `serde_json::Value`, already deserialized
/// by the runtime based on the message's schema.
///
/// Access fields through the provided accessor methods.
#[derive(Debug, Clone)]
pub struct SinkRecord {
    /// The actual message payload (typed data, already deserialized)
    pub(crate) payload: Value,
    /// User-defined attributes/properties from producer
    pub(crate) attributes: HashMap<String, String>,
    /// Danube metadata for observability and debugging
    pub(crate) danube_metadata: DanubeMetadata,
    /// Topic partition (if topic is partitioned)
    #[allow(dead_code)]
    pub(crate) partition: Option<String>,
    /// Schema information (if message has schema)
    pub(crate) schema_info: Option<SchemaInfo>,
}

/// Danube-specific metadata for observability and debugging
///
/// **Mandatory public API** - accessed through `SinkRecord` methods.
///
/// Provides message metadata like topic, offset, timestamps for observability.
#[derive(Debug, Clone)]
pub struct DanubeMetadata {
    /// Topic name
    pub(crate) topic: String,
    /// Message offset within topic
    pub(crate) offset: u64,
    /// Publish timestamp (microseconds since epoch)
    pub(crate) publish_time: u64,
    /// Formatted message ID for logging/debugging
    pub(crate) message_id: String,
    /// Producer name (for debugging)
    pub(crate) producer_name: String,
}

impl SinkRecord {
    /// Convert Danube StreamMessage to SinkRecord with schema-aware deserialization
    ///
    /// Fetches schema from registry (with caching) and deserializes the payload
    /// into a serde_json::Value based on the schema type.
    pub(crate) async fn from_stream_message(
        message: &StreamMessage,
        expected_schema_subject: &Option<String>,
        context: &ConnectorContext,
    ) -> ConnectorResult<Self> {
        // Deserialize payload based on schema
        let (payload, schema_info) =
            Self::deserialize_payload(message, expected_schema_subject, context).await?;

        // Build message ID for debugging
        let message_id = format!(
            "topic:{}/producer:{}/offset:{}",
            message.msg_id.topic_name, message.msg_id.producer_id, message.msg_id.topic_offset
        );

        Ok(SinkRecord {
            payload,
            attributes: message.attributes.clone(),
            danube_metadata: DanubeMetadata {
                topic: message.msg_id.topic_name.clone(),
                offset: message.msg_id.topic_offset,
                publish_time: message.publish_time,
                message_id,
                producer_name: message.producer_name.clone(),
            },
            partition: None, // TODO: Extract from message when partitioning is supported
            schema_info,
        })
    }

    /// Deserialize message payload based on schema
    async fn deserialize_payload(
        message: &StreamMessage,
        expected_schema_subject: &Option<String>,
        context: &ConnectorContext,
    ) -> ConnectorResult<(Value, Option<SchemaInfo>)> {
        if let Some(schema_id) = message.schema_id {
            // Message has schema - fetch and deserialize accordingly
            let schema = context.get_schema(schema_id).await?;

            // Validate expected schema if configured
            if let Some(expected) = expected_schema_subject {
                if &schema.subject != expected {
                    return Err(ConnectorError::invalid_data(
                        format!(
                            "Schema mismatch: expected '{}', got '{}' (schema_id: {})",
                            expected, schema.subject, schema_id
                        ),
                        Vec::new(),
                    ));
                }
            }

            let payload = match schema.schema_type.to_lowercase().as_str() {
                "json_schema" | "json" => {
                    serde_json::from_slice(&message.payload).map_err(|e| {
                        ConnectorError::invalid_data(
                            format!("JSON deserialization failed: {}", e),
                            message.payload.clone(),
                        )
                    })?
                }
                "string" => {
                    let s = std::str::from_utf8(&message.payload).map_err(|e| {
                        ConnectorError::invalid_data(
                            format!("UTF-8 decode failed: {}", e),
                            message.payload.clone(),
                        )
                    })?;
                    json!(s)
                }
                "number" => serde_json::from_slice(&message.payload).map_err(|e| {
                    ConnectorError::invalid_data(
                        format!("Number deserialization failed: {}", e),
                        message.payload.clone(),
                    )
                })?,
                "bytes" => {
                    // Bytes - encode as base64 in JSON
                    json!({
                        "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload),
                        "size": message.payload.len()
                    })
                }
                "avro" => {
                    // TODO: Implement Avro deserialization
                    return Err(ConnectorError::config(
                        "Avro deserialization not yet implemented",
                    ));
                }
                "protobuf" => {
                    // TODO: Implement Protobuf deserialization
                    return Err(ConnectorError::config(
                        "Protobuf deserialization not yet implemented",
                    ));
                }
                _ => {
                    // Unknown type - try JSON
                    warn!(
                        "Unknown schema type '{}', attempting JSON deserialization",
                        schema.schema_type
                    );
                    serde_json::from_slice(&message.payload).unwrap_or_else(|_| {
                        json!({
                            "raw": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload),
                            "size": message.payload.len()
                        })
                    })
                }
            };

            Ok((payload, Some(schema)))
        } else {
            // No schema - try JSON, fallback to base64
            let payload = serde_json::from_slice(&message.payload).unwrap_or_else(|_| {
                debug!("Message has no schema and is not JSON, encoding as base64");
                json!({
                    "raw": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload),
                    "size": message.payload.len()
                })
            });

            Ok((payload, None))
        }
    }

    /// Create a simple SinkRecord for testing (without schema registry)
    #[cfg(test)]
    pub(crate) fn new_for_test(message: StreamMessage) -> Self {
        let message_id = format!(
            "topic:{}/producer:{}/offset:{}",
            message.msg_id.topic_name, message.msg_id.producer_id, message.msg_id.topic_offset
        );

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
            partition: None,
            schema_info: None,
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
        let record = SinkRecord::new_for_test(message);

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

        let record = SinkRecord::new_for_test(message);
        let decoded: TestData = record.as_type().unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn test_sink_record_payload_access() {
        let mut message = create_test_message();
        let data = json!({"name": "test", "value": 42});
        message.payload = serde_json::to_vec(&data).unwrap();

        let record = SinkRecord::new_for_test(message);

        // Direct access to payload
        assert_eq!(record.payload()["name"], "test");
        assert_eq!(record.payload()["value"], 42);
    }

    #[test]
    fn test_sink_record_schema_info() {
        let message = create_test_message();
        let record = SinkRecord::new_for_test(message);

        // No schema info by default (runtime will set it)
        assert!(record.schema().is_none());
    }

    #[test]
    fn test_sink_record_attributes() {
        let mut message = create_test_message();
        message
            .attributes
            .insert("key1".to_string(), "value1".to_string());

        let record = SinkRecord::new_for_test(message);

        assert_eq!(record.get_attribute("key1"), Some("value1"));
        assert_eq!(record.get_attribute("key2"), None);
        assert!(record.has_attribute("key1"));
        assert!(!record.has_attribute("key2"));
    }
}
