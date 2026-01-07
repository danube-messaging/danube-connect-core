//! Integration test for programmatic configuration
//!
//! Tests that connectors can be configured entirely in code without TOML files.

use danube_connect_core::{
    ConnectorConfig, ProcessingSettings, RetrySettings, SchemaMapping, SubscriptionType,
    VersionStrategy,
};
use std::path::PathBuf;

#[test]
fn test_programmatic_connector_config() {
    // Create connector config entirely in code
    let config = ConnectorConfig {
        danube_service_url: "http://localhost:6650".to_string(),
        connector_name: "test-connector".to_string(),
        retry: RetrySettings {
            max_retries: 5,
            retry_backoff_ms: 2000,
            max_backoff_ms: 60000,
        },
        processing: ProcessingSettings {
            batch_size: 500,
            batch_timeout_ms: 2000,
            poll_interval_ms: 50,
            metrics_port: 9091,
            log_level: "debug".to_string(),
        },
        schemas: vec![],
    };

    // Validate fields
    assert_eq!(config.danube_service_url, "http://localhost:6650");
    assert_eq!(config.connector_name, "test-connector");
    assert_eq!(config.retry.max_retries, 5);
    assert_eq!(config.processing.batch_size, 500);
    assert_eq!(config.processing.metrics_port, 9091);
}

#[test]
fn test_programmatic_schema_config() {
    // Create schema mappings programmatically
    let schema_mappings = vec![
        SchemaMapping {
            topic: "/events/users".to_string(),
            subject: "user-events-v1".to_string(),
            schema_type: "json_schema".to_string(),
            schema_file: PathBuf::from("schemas/user.json"),
            version_strategy: VersionStrategy::Latest,
            auto_register: true,
        },
        SchemaMapping {
            topic: "/events/orders".to_string(),
            subject: "order-events-v2".to_string(),
            schema_type: "json_schema".to_string(),
            schema_file: PathBuf::from("schemas/order.json"),
            version_strategy: VersionStrategy::Pinned(3),
            auto_register: false,
        },
        SchemaMapping {
            topic: "/raw/telemetry".to_string(),
            subject: "telemetry-bytes".to_string(),
            schema_type: "bytes".to_string(),
            schema_file: PathBuf::from(""), // Empty path for bytes type
            version_strategy: VersionStrategy::Minimum(1),
            auto_register: true,
        },
    ];

    // Create full config with schemas
    let config = ConnectorConfig {
        danube_service_url: "http://localhost:6650".to_string(),
        connector_name: "schema-test-connector".to_string(),
        retry: RetrySettings::default(),
        processing: ProcessingSettings::default(),
        schemas: schema_mappings.clone(),
    };

    // Validate schema configurations
    assert_eq!(config.schemas.len(), 3);

    // Check first schema (Latest strategy)
    assert_eq!(config.schemas[0].topic, "/events/users");
    assert_eq!(config.schemas[0].subject, "user-events-v1");
    assert_eq!(config.schemas[0].schema_type, "json_schema");
    assert!(config.schemas[0].auto_register);
    assert!(matches!(
        config.schemas[0].version_strategy,
        VersionStrategy::Latest
    ));

    // Check second schema (Pinned strategy)
    assert_eq!(config.schemas[1].topic, "/events/orders");
    assert!(!config.schemas[1].auto_register);
    assert!(matches!(
        config.schemas[1].version_strategy,
        VersionStrategy::Pinned(3)
    ));

    // Check third schema (Bytes type, Minimum strategy)
    assert_eq!(config.schemas[2].schema_type, "bytes");
    assert!(matches!(
        config.schemas[2].version_strategy,
        VersionStrategy::Minimum(1)
    ));
}

#[test]
fn test_programmatic_retry_settings() {
    // Custom retry configuration
    let retry = RetrySettings {
        max_retries: 10,
        retry_backoff_ms: 500,
        max_backoff_ms: 120000,
    };

    assert_eq!(retry.max_retries, 10);
    assert_eq!(retry.retry_backoff_ms, 500);
    assert_eq!(retry.max_backoff_ms, 120000);

    // Default retry configuration
    let default_retry = RetrySettings::default();
    assert_eq!(default_retry.max_retries, 3);
    assert_eq!(default_retry.retry_backoff_ms, 1000);
    assert_eq!(default_retry.max_backoff_ms, 30000);
}

#[test]
fn test_programmatic_processing_settings() {
    // Custom processing settings
    let processing = ProcessingSettings {
        batch_size: 2000,
        batch_timeout_ms: 5000,
        poll_interval_ms: 200,
        metrics_port: 8080,
        log_level: "trace".to_string(),
    };

    assert_eq!(processing.batch_size, 2000);
    assert_eq!(processing.batch_timeout_ms, 5000);
    assert_eq!(processing.poll_interval_ms, 200);
    assert_eq!(processing.metrics_port, 8080);
    assert_eq!(processing.log_level, "trace");

    // Default processing settings
    let default_processing = ProcessingSettings::default();
    assert_eq!(default_processing.batch_size, 1000);
    assert_eq!(default_processing.batch_timeout_ms, 1000);
    assert_eq!(default_processing.poll_interval_ms, 100);
    assert_eq!(default_processing.metrics_port, 9090);
    assert_eq!(default_processing.log_level, "info");
}

#[test]
fn test_subscription_type_variants() {
    // Test all subscription type variants can be created
    let exclusive = SubscriptionType::Exclusive;
    let shared = SubscriptionType::Shared;
    let failover = SubscriptionType::FailOver;

    // Validate they're distinct
    assert!(matches!(exclusive, SubscriptionType::Exclusive));
    assert!(matches!(shared, SubscriptionType::Shared));
    assert!(matches!(failover, SubscriptionType::FailOver));
}

#[test]
fn test_version_strategy_variants() {
    // Test all version strategy variants
    let latest = VersionStrategy::Latest;
    let pinned = VersionStrategy::Pinned(5);
    let minimum = VersionStrategy::Minimum(2);

    // Validate they work correctly
    assert!(matches!(latest, VersionStrategy::Latest));

    if let VersionStrategy::Pinned(version) = pinned {
        assert_eq!(version, 5);
    } else {
        panic!("Expected Pinned variant");
    }

    if let VersionStrategy::Minimum(version) = minimum {
        assert_eq!(version, 2);
    } else {
        panic!("Expected Minimum variant");
    }
}

#[test]
fn test_full_connector_config_with_all_features() {
    // Create a comprehensive connector configuration programmatically
    let config = ConnectorConfig {
        danube_service_url: "http://danube-cluster:6650".to_string(),
        connector_name: "production-connector".to_string(),
        retry: RetrySettings {
            max_retries: 5,
            retry_backoff_ms: 2000,
            max_backoff_ms: 60000,
        },
        processing: ProcessingSettings {
            batch_size: 1000,
            batch_timeout_ms: 2000,
            poll_interval_ms: 100,
            metrics_port: 9090,
            log_level: "info".to_string(),
        },
        schemas: vec![
            SchemaMapping {
                topic: "/production/events".to_string(),
                subject: "prod-events-schema".to_string(),
                schema_type: "json_schema".to_string(),
                schema_file: PathBuf::from("schemas/production/events.json"),
                version_strategy: VersionStrategy::Pinned(5),
                auto_register: false,
            },
            SchemaMapping {
                topic: "/production/logs".to_string(),
                subject: "logs".to_string(),
                schema_type: "string".to_string(),
                schema_file: PathBuf::from(""),
                version_strategy: VersionStrategy::Latest,
                auto_register: true,
            },
        ],
    };

    // Comprehensive validation
    assert_eq!(config.connector_name, "production-connector");
    assert_eq!(config.schemas.len(), 2);
    assert_eq!(config.retry.max_retries, 5);
    assert_eq!(config.processing.batch_size, 1000);

    // Validate specific schema
    let events_schema = &config.schemas[0];
    assert_eq!(events_schema.topic, "/production/events");
    assert_eq!(events_schema.subject, "prod-events-schema");
    assert!(!events_schema.auto_register);
}

#[test]
fn test_config_builder_pattern() {
    // Demonstrate programmatic config can be built incrementally
    let mut config = ConnectorConfig {
        danube_service_url: "http://localhost:6650".to_string(),
        connector_name: "builder-test".to_string(),
        retry: RetrySettings::default(),
        processing: ProcessingSettings::default(),
        schemas: vec![],
    };

    // Add schemas incrementally
    config.schemas.push(SchemaMapping {
        topic: "/test/topic1".to_string(),
        subject: "topic1-schema".to_string(),
        schema_type: "json_schema".to_string(),
        schema_file: PathBuf::from("schema1.json"),
        version_strategy: VersionStrategy::Latest,
        auto_register: true,
    });

    config.schemas.push(SchemaMapping {
        topic: "/test/topic2".to_string(),
        subject: "topic2-schema".to_string(),
        schema_type: "string".to_string(),
        schema_file: PathBuf::from(""),
        version_strategy: VersionStrategy::Latest,
        auto_register: true,
    });

    assert_eq!(config.schemas.len(), 2);
    assert_eq!(config.schemas[0].topic, "/test/topic1");
    assert_eq!(config.schemas[1].topic, "/test/topic2");
}
