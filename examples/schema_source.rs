//! Schema-aware source connector example
//!
//! This example demonstrates schema registry integration with a source connector.
//! It uses SchemaType::String which doesn't require a schema file.
//!
//! Usage:
//!   DANUBE_SERVICE_URL=http://localhost:6650 \
//!   CONNECTOR_NAME=schema-source \
//!   cargo run --example schema_source

use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ProcessingSettings, ProducerConfig, RetrySettings,
    SchemaMapping, SourceConnector, SourceRecord, SourceRuntime, VersionStrategy,
};
use std::time::Duration;

/// A schema-aware source connector that generates test messages
struct SchemaSourceConnector {
    counter: u64,
    max_messages: u64,
}

impl SchemaSourceConnector {
    fn new(max_messages: u64) -> Self {
        Self {
            counter: 0,
            max_messages,
        }
    }
}

#[async_trait]
impl SourceConnector for SchemaSourceConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("SchemaSourceConnector initialized");
        println!("Configuration: {:?}", config);
        println!(
            "Will generate {} messages with String schema",
            self.max_messages
        );
        Ok(())
    }

    async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>> {
        // Define producer with schema configuration
        // NOTE: schema_config will be populated by runtime from ConnectorConfig.schemas
        Ok(vec![ProducerConfig {
            topic: "/default/test-schema".to_string(),
            partitions: 0,
            reliable_dispatch: true,
            schema_config: None, // Runtime will populate from config.schemas
        }])
    }

    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
        // Check if we've reached the limit
        if self.counter >= self.max_messages {
            tokio::time::sleep(Duration::from_secs(1)).await;
            return Ok(vec![]);
        }

        // Generate a batch of messages
        let mut records = Vec::new();
        let batch_size = 5.min(self.max_messages - self.counter);

        for i in 0..batch_size {
            self.counter += 1;

            let message = format!(
                "Schema message #{} - Timestamp: {}",
                self.counter,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );

            // Use from_string() to create typed string data
            let record = SourceRecord::from_string("/default/test-schema", message)
                .with_attribute("source", "schema-source-connector")
                .with_attribute("message_number", self.counter.to_string())
                .with_attribute("batch_index", i.to_string());

            records.push(record);

            println!("Generated schema message #{}", self.counter);
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(records)
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        println!("SchemaSourceConnector shutting down");
        println!("Total messages generated: {}", self.counter);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // Configuration with schema registry integration
    let config = ConnectorConfig::from_env().unwrap_or_else(|_| {
        println!("Using configuration with schema registry");
        println!("To use custom settings, set environment variables:");
        println!("  DANUBE_SERVICE_URL (default: http://localhost:6650)");
        println!("  CONNECTOR_NAME (default: schema-source)");
        println!();

        ConnectorConfig {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "schema-source".to_string(),
            retry: RetrySettings::default(),
            processing: ProcessingSettings::default(),
            // Schema mapping for the topic
            schemas: vec![SchemaMapping {
                topic: "/default/test-schema".to_string(),
                subject: "test-schema-value".to_string(),
                schema_type: "string".to_string(), // Primitive types: string, number, bytes
                schema_file: "".into(), // Empty for primitive types (only needed for json_schema, avro, protobuf)
                auto_register: true,    // Auto-register with schema registry
                version_strategy: VersionStrategy::Latest,
            }],
        }
    });

    println!("=== Schema Configuration ===");
    println!("Subject: test-schema-value");
    println!("Type: String (no schema file needed)");
    println!("Auto-register: enabled");
    println!("Version strategy: Latest");
    println!();

    // Create connector instance (generate 50 messages)
    let connector = SchemaSourceConnector::new(50);

    // Create and run runtime
    let mut runtime = SourceRuntime::new(connector, config).await?;
    runtime.run().await
}
