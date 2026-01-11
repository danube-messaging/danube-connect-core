//! Schema-aware sink connector example
//!
//! This example demonstrates schema registry integration with a sink connector.
//! It validates that incoming messages match the expected String schema.
//!
//! Usage:
//!   DANUBE_SERVICE_URL=http://localhost:6650 \
//!   CONNECTOR_NAME=schema-sink \
//!   cargo run --example schema_sink

use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ConsumerConfig, ProcessingSettings, RetrySettings,
    SinkConnector, SinkRecord, SinkRuntime, SubscriptionType,
};

/// A schema-aware sink connector that validates schema types
struct SchemaSinkConnector {
    message_count: u64,
}

impl SchemaSinkConnector {
    fn new() -> Self {
        Self { message_count: 0 }
    }
}

#[async_trait]
impl SinkConnector for SchemaSinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("SchemaSinkConnector initialized");
        println!("Configuration: {:?}", config);
        println!("Expecting String schema from messages");
        Ok(())
    }

    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
        // Configure consumer with expected schema validation
        Ok(vec![ConsumerConfig {
            topic: "/default/test-schema".to_string(),
            consumer_name: "schema-sink-consumer".to_string(),
            subscription: "schema-sink-sub".to_string(),
            subscription_type: SubscriptionType::Exclusive,
            // Validate that messages match this schema subject
            expected_schema_subject: Some("test-schema-value".to_string()),
        }])
    }

    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        self.message_count += 1;

        println!("=== Message #{} ===", self.message_count);
        println!("Topic: {}", record.topic());
        println!("Producer: {}", record.producer_name());
        println!("Publish Time: {}", record.publish_time());

        // Check schema - should always be present with schema registry
        match record.schema() {
            Some(schema) => {
                println!("✓ Schema validated!");
                println!("  Subject: {}", schema.subject);
                println!("  Version: {}", schema.version);
                println!("  Type: {}", schema.schema_type);
                
                // Verify it's the expected type
                if schema.schema_type.to_lowercase() != "string" {
                    println!("⚠ WARNING: Expected String schema, got {}", schema.schema_type);
                }
            }
            None => {
                println!("⚠ WARNING: No schema found! Expected test-schema-value");
            }
        }

        // Payload is already deserialized by runtime based on schema type
        let payload = record.payload();
        
        // Since schema type is String, payload should be a JSON string
        if let Some(text) = payload.as_str() {
            println!("Payload: {}", text);
        } else {
            println!("⚠ Payload type mismatch! Expected string, got: {:?}", payload);
        }

        // Print attributes
        if !record.attributes().is_empty() {
            println!("Attributes:");
            for (key, value) in record.attributes() {
                println!("  {} = {}", key, value);
            }
        }

        println!();

        Ok(())
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        println!("SchemaSinkConnector shutting down");
        println!("Total messages processed: {}", self.message_count);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    let config = ConnectorConfig::from_env().unwrap_or_else(|_| {
        println!("Using configuration with schema validation");
        println!("To use custom settings, set environment variables:");
        println!("  DANUBE_SERVICE_URL (default: http://localhost:6650)");
        println!("  CONNECTOR_NAME (default: schema-sink)");
        println!();

        ConnectorConfig {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "schema-sink".to_string(),
            retry: RetrySettings::default(),
            processing: ProcessingSettings::default(),
            schemas: Vec::new(), // Sink doesn't need to register schemas
        }
    });

    println!("=== Schema Validation Settings ===");
    println!("Expected subject: test-schema-value");
    println!("Expected type: String");
    println!("Validation: enabled (runtime will reject mismatched schemas)");
    println!();

    // Create connector instance
    let connector = SchemaSinkConnector::new();

    // Create and run runtime
    let mut runtime = SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
