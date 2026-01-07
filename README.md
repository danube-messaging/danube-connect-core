# danube-connect-core

[![Crates.io](https://img.shields.io/crates/v/danube-connect-core.svg)](https://crates.io/crates/danube-connect-core)
[![Docs.rs](https://docs.rs/danube-connect-core/badge.svg)](https://docs.rs/danube-connect-core)
[![License](https://img.shields.io/crates/l/danube-connect-core.svg)](https://github.com/danube-messaging/danube-connect/blob/main/LICENSE)

Core SDK for building high-performance connectors for the [Danube messaging system](https://github.com/danube-messaging/danube).

## Overview

`danube-connect-core` is a production-ready framework for building connectors that integrate Danube with external systems. Whether you're building a sink connector to export messages or a source connector to import data, this SDK provides everything you need.

### Key Features

- **Simple Trait-Based API** - Implement just `SinkConnector` or `SourceConnector` traits
- **Multi-Topic Support** - Sink connectors can consume from multiple topics, source connectors can publish to multiple topics dynamically
- **Flexible Topic Configuration** - Connectors specify their own topic/subscription settings
- **Automatic Runtime Management** - Lifecycle handling, message loops, and graceful shutdown
- **Built-in Retry Logic** - Exponential backoff with jitter for resilient integrations
- **Observability** - Prometheus metrics, structured logging, and health checks
- **Message Utilities** - Batching, serialization, and format conversion helpers
- **Hybrid Configuration** - Mandatory fields from env vars, optional settings from TOML files
- **Error Handling** - Comprehensive error types and recovery strategies
- **Async/Await** - Built on Tokio for high-performance async operations

## Core Concepts

### Traits

- **`SinkConnector`** - Consumes messages from Danube and sends them to an external system
  - Implement `consumer_configs()` to specify which topics to consume from
  - Implement `process()` to handle individual messages
  - Supports multiple topic consumption
  
- **`SourceConnector`** - Reads data from an external system and publishes to Danube
  - Implement `poll()` to read data and create `SourceRecord`s
  - Use `ProducerConfig` to specify per-record topic and partitioning
  - Supports dynamic multi-topic publishing

### Runtime

- **`SinkRuntime`** - Manages the lifecycle of a sink connector
  - Creates multiple consumers based on `consumer_configs()`
  - Handles message processing with retry logic
  - Manages acknowledgments and dead-letter queues
  
- **`SourceRuntime`** - Manages the lifecycle of a source connector
  - Creates producers dynamically based on `ProducerConfig`
  - Handles batching and publishing
  - Manages offsets and checkpointing

Both runtimes handle:
- Danube client connection management
- Message polling/processing loops
- Error recovery and automatic retries
- Graceful shutdown
- Signal handling (SIGTERM, SIGINT)
- Prometheus metrics and health checks

### Configuration

Configuration uses a hybrid approach:
- **Mandatory fields** from environment variables
- **Optional settings** from TOML config files with sensible defaults

#### Environment Variables

Only mandatory deployment-specific fields:

```bash
# Required
DANUBE_SERVICE_URL=http://localhost:6650  # Danube broker URL
CONNECTOR_NAME=my-connector              # Unique connector name
```

#### TOML Configuration File

Optional settings for retry and processing behavior:

```toml
# Mandatory fields (can also be set via env vars)
danube_service_url = "http://localhost:6650"
connector_name = "my-connector"

# Retry settings (optional, these are defaults)
[retry]
max_retries = 3
retry_backoff_ms = 1000
max_backoff_ms = 30000

# Processing settings (optional, these are defaults)
[processing]
batch_size = 1000
batch_timeout_ms = 1000
poll_interval_ms = 100
metrics_port = 9090
log_level = "info"
```

Load config:

```rust
// From environment only (uses defaults for retry/processing)
let config = ConnectorConfig::from_env()?;

// From TOML file (with env var overrides)
let mut config = ConnectorConfig::from_file("connector.toml")?;
config.apply_env_overrides();

// Programmatically
let config = ConnectorConfig {
    danube_service_url: "http://localhost:6650".to_string(),
    connector_name: "my-connector".to_string(),
    retry: RetrySettings::default(),
    processing: ProcessingSettings::default(),
};
```

#### Topic Configuration

**Sink connectors** specify topics via `consumer_configs()` method:
```rust
async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
    Ok(vec![ConsumerConfig {
        topic: "/default/my-topic".to_string(),
        consumer_name: "my-consumer".to_string(),
        subscription: "my-sub".to_string(),
        subscription_type: SubscriptionType::Exclusive,
    }])
}
```

**Source connectors** specify topics per record via `ProducerConfig`:
```rust
let record = SourceRecord::from_string("data")
    .with_producer_config(ProducerConfig {
        topic: "/default/my-topic".to_string(),
        partitions: 4,
        reliable_dispatch: true,
    });
```

### Message Types

- **`SinkRecord`** - Message received from Danube (topic, offset, payload, attributes)
- **`SourceRecord`** - Message to publish to Danube (topic, payload, attributes)

### Utilities

The SDK includes helpful utilities in the `utils` module:

- **`Batcher<T>`** - Collect messages with size/timeout-based flushing
- **`HealthChecker`** - Track connector health with failure thresholds
- **`serialization`** - JSON and string conversion helpers

```rust
use danube_connect_core::{Batcher, HealthChecker};
use std::time::Duration;

// Batch messages
let mut batcher = Batcher::new(100, Duration::from_secs(5));
batcher.add(record);
if batcher.should_flush() {
    let batch = batcher.flush();
    // Process batch
}

// Track health
let mut health = HealthChecker::new(3); // Unhealthy after 3 failures
health.record_failure();
if !health.is_healthy() {
    // Handle degraded state
}
```

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
danube-connect-core = "0.3"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

### Building Connectors

**For detailed implementation guides, see:**
- **[Connector Development Guide](./info/connector-development-guide.md)** - Comprehensive guide on implementing sink and source connectors
- **[Reference Connectors](https://github.com/danube-messaging/danube-connectors)** - Production-ready connector implementations

## Examples

The repository includes working examples:

- **`simple_sink`** - Basic sink connector that prints messages
- **`simple_source`** - Basic source connector that generates test messages

Run them with Docker:

```bash
# Start Danube cluster (if you have the docker setup)
cd docker
docker-compose up -d

# Run examples
cargo run --example simple_sink
cargo run --example simple_source
```

## Testing

Test your connector with a local Danube cluster:

```bash
# Start a local Danube broker
docker run -p 6650:6650 ghcr.io/danube-messaging/danube:latest

# Run your connector
cargo run --bin my-connector

# Monitor metrics (default port: 9090)
curl http://localhost:9090/metrics
```

For complete production examples, see the [danube-connectors](https://github.com/danube-messaging/danube-connectors) repository.

## Documentation

### Connector Documentation
- **[Connector Development Guide](./info/connector-development-guide.md)** - Conceptual guide for implementing connectors
- **[Architecture Overview](./info/connectors.md)** - Connector framework architecture and design
- **[Message Patterns](./info/connector-message-patterns.md)** - Message transformation and data flow patterns
- **[Configuration Guide](./info/unified_configuration_guide.md)** - Configuration architecture and best practices

### Reference Implementations
- **[Production Connectors](https://github.com/danube-messaging/danube-connectors)** - Complete reference implementations
  - MQTT Source Connector
  - HTTP/Webhook Source Connector
  - Qdrant Sink Connector
  - SurrealDB Sink Connector
  - Delta Lake Sink Connector

## Performance

`danube-connect-core` is designed for high-performance integrations:

- **Async/await** - Built on Tokio for efficient async operations
- **Batching** - Process messages in batches for better throughput
- **Connection pooling** - Reuse connections to external systems
- **Metrics** - Monitor performance with Prometheus


## Contributing

Contributions are welcome! Here's how you can help:

- **Core SDK Improvements**: Enhance the connector framework, add new features, or improve performance
- **Documentation**: Improve guides, add examples, or clarify concepts
- **Bug Fixes**: Report and fix issues
- **Testing**: Add test coverage and integration tests

Please open an issue or pull request on GitHub.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](./LICENSE) for details.

## Related Projects

- **[Danube](https://github.com/danube-messaging/danube)** - The core messaging system
- **[Danube Connectors](https://github.com/danube-messaging/danube-connectors)** - Production-ready connector implementations
- **[Danube Docs](https://danube-docs.dev-state.com)** - Official documentation

## Support

- **Crates.io**: [danube-connect-core](https://crates.io/crates/danube-connect-core)
- **Docs.rs**: [API Documentation](https://docs.rs/danube-connect-core)
- **GitHub Issues**: [Report bugs or request features](https://github.com/danube-messaging/danube-connect-core/issues)
