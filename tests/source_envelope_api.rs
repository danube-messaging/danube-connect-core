use danube_connect_core::{Offset, SourceEnvelope, SourceRecord};
use serde_json::json;

#[test]
fn test_source_envelope_with_offset_roundtrip() {
    let record = SourceRecord::new("/default/events", json!({"value": 1}))
        .with_attribute("source", "test-suite")
        .with_key("user-1");
    let offset = Offset::with_metadata("partition-1", 42, "checkpoint-a");

    let envelope = SourceEnvelope::with_offset(record, offset.clone());
    assert_eq!(envelope.record().topic(), "/default/events");
    assert_eq!(envelope.record().key(), Some("user-1"));
    assert_eq!(
        envelope.record().attributes().get("source").map(String::as_str),
        Some("test-suite")
    );
    assert_eq!(envelope.offset(), Some(&offset));

    let (record, roundtrip_offset) = envelope.into_parts();
    assert_eq!(record.topic(), "/default/events");
    assert_eq!(record.key(), Some("user-1"));
    assert_eq!(roundtrip_offset, Some(offset));
}

#[test]
fn test_source_envelope_from_record_has_no_offset() {
    let record = SourceRecord::from_string("/default/logs", "hello world");
    let envelope = SourceEnvelope::from(record);

    assert_eq!(envelope.record().topic(), "/default/logs");
    assert!(envelope.offset().is_none());
}

#[test]
fn test_source_record_context_public_api() {
    let record = SourceRecord::new("/default/metrics", json!({"count": 3}))
        .with_attribute("service", "api")
        .with_key("tenant-7");

    let routing = record.routing_context();
    assert_eq!(routing.topic(), "/default/metrics");
    assert_eq!(routing.key(), Some("tenant-7"));
    assert_eq!(routing.partition(), None);
    assert_eq!(
        routing.attributes().get("service").map(String::as_str),
        Some("api")
    );

    let context = record.context();
    assert_eq!(context.topic(), "/default/metrics");
    assert_eq!(context.key(), Some("tenant-7"));
    assert_eq!(context.partition(), None);
    assert_eq!(context.publish_time(), None);
    assert_eq!(context.producer_name(), None);
    assert!(context.schema().is_none());
}
