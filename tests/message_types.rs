//! Integration test for message types
//!
//! Tests SourceRecord creation and builder pattern without requiring a Danube broker.
//!
//! Note: Serialization tests are not included here because `serialize_with_schema`
//! is an internal method (`pub(crate)`). It's tested indirectly through the runtime
//! in the examples and end-to-end tests.

use danube_connect_core::SourceRecord;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_source_record_from_json_value() {
    // Create SourceRecord from JSON value
    let payload = json!({
        "user_id": "123",
        "action": "login",
        "timestamp": 1234567890
    });

    let record = SourceRecord::new("/events/users", payload.clone());

    assert_eq!(record.topic, "/events/users");
    assert_eq!(record.payload(), &payload);
    assert!(record.key.is_none());
    assert!(record.attributes.is_empty());
}

#[test]
fn test_source_record_from_struct() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct UserEvent {
        user_id: String,
        action: String,
        timestamp: u64,
    }

    let event = UserEvent {
        user_id: "user-123".to_string(),
        action: "signup".to_string(),
        timestamp: 1234567890,
    };

    // Create from JSON-serializable struct
    let record = SourceRecord::from_json("/events/users", &event)
        .expect("Failed to create SourceRecord from struct");

    assert_eq!(record.topic, "/events/users");

    // Verify payload matches
    let payload = record.payload();
    assert_eq!(payload["user_id"], "user-123");
    assert_eq!(payload["action"], "signup");
    assert_eq!(payload["timestamp"], 1234567890);
}

#[test]
fn test_source_record_from_string() {
    let text = "This is a log message";
    let record = SourceRecord::from_string("/logs/application", text);

    assert_eq!(record.topic, "/logs/application");
    assert_eq!(record.payload().as_str().unwrap(), text);
}

#[test]
fn test_source_record_with_attributes() {
    let payload = json!({"data": "test"});

    let record = SourceRecord::new("/test/topic", payload)
        .with_attribute("source", "test-system")
        .with_attribute("priority", "high")
        .with_attribute("version", "1.0");

    assert_eq!(record.attributes.len(), 3);
    assert_eq!(record.attributes.get("source").unwrap(), "test-system");
    assert_eq!(record.attributes.get("priority").unwrap(), "high");
    assert_eq!(record.attributes.get("version").unwrap(), "1.0");
}

#[test]
fn test_source_record_with_multiple_attributes() {
    let mut attrs = HashMap::new();
    attrs.insert("region".to_string(), "us-east-1".to_string());
    attrs.insert("env".to_string(), "production".to_string());
    attrs.insert("service".to_string(), "api".to_string());

    let payload = json!({"event": "test"});
    let record = SourceRecord::new("/events/metrics", payload).with_attributes(attrs);

    assert_eq!(record.attributes.len(), 3);
    assert_eq!(record.attributes.get("region").unwrap(), "us-east-1");
    assert_eq!(record.attributes.get("env").unwrap(), "production");
    assert_eq!(record.attributes.get("service").unwrap(), "api");
}

#[test]
fn test_source_record_with_routing_key() {
    let payload = json!({"user_id": "123", "data": "test"});
    let record = SourceRecord::new("/partitioned/topic", payload).with_key("user-123");

    assert_eq!(record.key.as_ref().unwrap(), "user-123");
}

#[test]
fn test_source_record_builder_pattern() {
    // Test chaining multiple builder methods
    let payload = json!({
        "order_id": "order-456",
        "amount": 99.99,
        "currency": "USD"
    });

    let record = SourceRecord::new("/orders/created", payload)
        .with_key("order-456")
        .with_attribute("customer_id", "cust-789")
        .with_attribute("region", "us-west")
        .with_attribute("payment_method", "credit_card");

    assert_eq!(record.topic, "/orders/created");
    assert_eq!(record.key.as_ref().unwrap(), "order-456");
    assert_eq!(record.attributes.len(), 3);
    assert_eq!(record.payload()["order_id"], "order-456");
    assert_eq!(record.payload()["amount"], 99.99);
}

#[test]
fn test_source_record_complex_nested_json() {
    // Test with complex nested structure
    let payload = json!({
        "user": {
            "id": "user-123",
            "profile": {
                "name": "Alice",
                "age": 28,
                "preferences": {
                    "theme": "dark",
                    "notifications": true
                }
            },
            "tags": ["premium", "verified"]
        },
        "metadata": {
            "timestamp": 1234567890,
            "source": "web"
        }
    });

    let record = SourceRecord::new("/users/activity", payload.clone())
        .with_key("user-123")
        .with_attribute("event_type", "profile_update");

    // Verify structure is preserved in payload
    assert_eq!(record.payload()["user"]["id"], "user-123");
    assert_eq!(record.payload()["user"]["profile"]["name"], "Alice");
    assert_eq!(record.payload()["user"]["tags"][0], "premium");
    assert_eq!(record.payload()["metadata"]["source"], "web");
    assert_eq!(record.key.as_ref().unwrap(), "user-123");
    assert_eq!(
        record.attributes.get("event_type").unwrap(),
        "profile_update"
    );
}

#[test]
fn test_source_record_empty_payload() {
    let payload = json!({});
    let record = SourceRecord::new("/test/empty", payload.clone());

    // Verify empty payload is handled correctly
    assert_eq!(record.payload(), &payload);
}

#[test]
fn test_source_record_array_payload() {
    let payload = json!([
        {"id": 1, "value": "first"},
        {"id": 2, "value": "second"},
        {"id": 3, "value": "third"}
    ]);

    let record = SourceRecord::new("/batch/data", payload.clone());

    // Verify array payload is preserved
    assert_eq!(record.payload(), &payload);
    assert_eq!(record.payload()[1]["value"], "second");
    assert_eq!(record.payload()[2]["id"], 3);
}

#[test]
fn test_multiple_records_with_different_topics() {
    // Simulate creating multiple records for different topics
    let records = vec![
        SourceRecord::new("/events/user", json!({"event": "login"})),
        SourceRecord::new("/events/order", json!({"event": "purchase"})),
        SourceRecord::new("/logs/system", json!({"level": "info"})),
    ];

    assert_eq!(records[0].topic, "/events/user");
    assert_eq!(records[1].topic, "/events/order");
    assert_eq!(records[2].topic, "/logs/system");

    assert_eq!(records[0].payload()["event"], "login");
    assert_eq!(records[1].payload()["event"], "purchase");
    assert_eq!(records[2].payload()["level"], "info");
}

#[test]
fn test_source_record_payload_ownership() {
    // Verify that creating a record doesn't prevent using the original value
    let payload = json!({"test": "data"});
    let payload_clone = payload.clone();

    let record = SourceRecord::new("/test/topic", payload);

    // Original payload (cloned) should still be usable
    assert_eq!(payload_clone["test"], "data");
    assert_eq!(record.payload()["test"], "data");
}
