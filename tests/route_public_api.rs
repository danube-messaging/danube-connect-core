use danube_connect_core::{
    ConsumerConfig, ProducerConfig, RouteDispatchPolicy, RouteSchemaPolicy,
    RouteSubscriptionPolicy, SchemaConfig, SinkRoute, SourceRoute, SubscriptionType,
    VersionStrategy,
};
use std::path::PathBuf;

#[test]
fn test_consumer_config_route_roundtrip_public_api() {
    let config = ConsumerConfig {
        topic: "/default/events".to_string(),
        consumer_name: "events-consumer".to_string(),
        subscription: "events-sub".to_string(),
        subscription_type: SubscriptionType::Shared,
        expected_schema_subject: Some("events-value".to_string()),
    };

    let route = config.route();
    assert_eq!(route.topic, "/default/events");
    assert_eq!(route.subscription.consumer_name, "events-consumer");
    assert_eq!(route.subscription.subscription, "events-sub");
    assert!(matches!(
        route.subscription.subscription_type,
        SubscriptionType::Shared
    ));
    assert_eq!(route.schema.expected_subject.as_deref(), Some("events-value"));
    assert!(route.schema.output_schema.is_none());

    let roundtrip = ConsumerConfig::from_route(route);
    assert_eq!(roundtrip.topic, config.topic);
    assert_eq!(roundtrip.consumer_name, config.consumer_name);
    assert_eq!(roundtrip.subscription, config.subscription);
    assert!(matches!(roundtrip.subscription_type, SubscriptionType::Shared));
    assert_eq!(
        roundtrip.expected_schema_subject,
        Some("events-value".to_string())
    );
}

#[test]
fn test_source_route_roundtrip_public_api() {
    let config = ProducerConfig {
        topic: "/default/output".to_string(),
        partitions: 4,
        reliable_dispatch: true,
        schema_config: Some(SchemaConfig {
            subject: "output-value".to_string(),
            schema_type: "json_schema".to_string(),
            schema_file: PathBuf::from("schemas/output.json"),
            auto_register: true,
            version_strategy: VersionStrategy::Pinned(2),
        }),
    };

    let route = config.route();
    assert_eq!(route.topic, "/default/output");
    assert_eq!(route.dispatch.partitions, 4);
    assert!(route.dispatch.reliable_dispatch);
    assert!(route.schema.expected_subject.is_none());
    assert_eq!(
        route.schema.output_schema.as_ref().map(|schema| schema.subject.as_str()),
        Some("output-value")
    );

    let roundtrip = ProducerConfig::from_route(route);
    assert_eq!(roundtrip.topic, config.topic);
    assert_eq!(roundtrip.partitions, config.partitions);
    assert_eq!(roundtrip.reliable_dispatch, config.reliable_dispatch);
    assert_eq!(
        roundtrip.schema_config.as_ref().map(|schema| schema.subject.as_str()),
        Some("output-value")
    );
}

#[test]
fn test_explicit_route_builders_convert_to_configs() {
    let sink_route = SinkRoute::new(
        "/default/sink",
        RouteSubscriptionPolicy::new("sink-consumer", "sink-sub", SubscriptionType::Exclusive),
        RouteSchemaPolicy {
            expected_subject: Some("sink-value".to_string()),
            output_schema: None,
        },
    );
    let consumer = ConsumerConfig::from_route(sink_route);
    assert_eq!(consumer.topic, "/default/sink");
    assert_eq!(consumer.consumer_name, "sink-consumer");
    assert_eq!(consumer.subscription, "sink-sub");
    assert!(matches!(consumer.subscription_type, SubscriptionType::Exclusive));
    assert_eq!(consumer.expected_schema_subject.as_deref(), Some("sink-value"));

    let source_route = SourceRoute::new(
        "/default/source",
        RouteDispatchPolicy::new(2, true),
        RouteSchemaPolicy {
            expected_subject: None,
            output_schema: None,
        },
    );
    let producer = ProducerConfig::from_route(source_route);
    assert_eq!(producer.topic, "/default/source");
    assert_eq!(producer.partitions, 2);
    assert!(producer.reliable_dispatch);
    assert!(producer.schema_config.is_none());
}
