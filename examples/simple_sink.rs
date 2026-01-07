//! Simple sink connector example
//!
//! This example demonstrates a minimal sink connector that prints messages to stdout.
//!
//! **NOTE:** This is a simplified example for testing the core library.
//! For production connectors, use the unified TOML+ENV configuration pattern.
//! See `connectors/source-mqtt/` for the recommended implementation.
//!
//! Usage:
//!   DANUBE_SERVICE_URL=http://localhost:6650 \
//!   CONNECTOR_NAME=simple-sink \
//!   DANUBE_TOPIC=/default/test \
//!   SUBSCRIPTION_NAME=simple-sub \
//!   cargo run --example simple_sink

use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ConsumerConfig, ProcessingSettings, RetrySettings,
    SinkConnector, SinkRecord, SinkRuntime, SubscriptionType,
};

/// A simple sink connector that prints messages
struct SimpleSinkConnector {
    message_count: u64,
}

impl SimpleSinkConnector {
    fn new() -> Self {
        Self { message_count: 0 }
    }
}

#[async_trait]
impl SinkConnector for SimpleSinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("SimpleSinkConnector initialized");
        println!("Configuration: {:?}", config);
        Ok(())
    }

    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
        // Simple example: consume from one topic
        Ok(vec![ConsumerConfig {
            topic: "/default/test".to_string(),
            consumer_name: "simple-sink-consumer".to_string(),
            subscription: "simple-sink-sub".to_string(),
            subscription_type: SubscriptionType::Exclusive,
            expected_schema_subject: None, // No schema validation for simple example
        }])
    }

    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        self.message_count += 1;

        println!("=== Message #{} ===", self.message_count);
        println!("Topic: {}", record.topic());
        println!("Offset: {}", record.offset());
        println!("Producer: {}", record.producer_name());
        println!("Publish Time: {}", record.publish_time());

        // NEW: Payload is now typed data (serde_json::Value)
        let payload = record.payload();

        // Check if message has schema
        if let Some(schema) = record.schema() {
            println!(
                "Schema: {} (version: {}, type: {})",
                schema.subject, schema.version, schema.schema_type
            );
        } else {
            println!("Schema: None (no schema registry used)");
        }

        // Print payload based on its type
        if let Some(text) = payload.as_str() {
            println!("Payload (string): {}", text);
        } else if let Some(obj) = payload.as_object() {
            println!(
                "Payload (JSON object): {}",
                serde_json::to_string_pretty(obj).unwrap()
            );
        } else if let Some(arr) = payload.as_array() {
            println!("Payload (JSON array): {} items", arr.len());
        } else if let Some(num) = payload.as_f64() {
            println!("Payload (number): {}", num);
        } else if let Some(b) = payload.as_bool() {
            println!("Payload (boolean): {}", b);
        } else {
            println!("Payload: {}", payload);
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
        println!("SimpleSinkConnector shutting down");
        println!("Total messages processed: {}", self.message_count);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // NOTE: This example uses simple ENV-only config for testing.
    // Production connectors should use unified TOML+ENV config (see connectors/source-mqtt/)
    let config = ConnectorConfig::from_env().unwrap_or_else(|_| {
        println!("Using default configuration for testing");
        println!("To use custom settings, set environment variables:");
        println!("  DANUBE_SERVICE_URL (default: http://localhost:6650)");
        println!("  CONNECTOR_NAME (default: simple-sink)");
        println!("  DANUBE_TOPIC (default: /default/test)");
        println!("  SUBSCRIPTION_NAME (default: simple-sink-sub)");
        println!();

        ConnectorConfig {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "simple-sink".to_string(),
            retry: RetrySettings::default(),
            processing: ProcessingSettings::default(),
            schemas: Vec::new(), // No schema configuration for simple example
        }
    });

    // Create connector instance
    let connector = SimpleSinkConnector::new();

    // Create and run runtime
    let mut runtime = SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
