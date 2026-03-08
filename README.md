# danube-connect-core

[![Crates.io](https://img.shields.io/crates/v/danube-connect-core.svg)](https://crates.io/crates/danube-connect-core)
[![Docs.rs](https://docs.rs/danube-connect-core/badge.svg)](https://docs.rs/danube-connect-core)
[![License](https://img.shields.io/crates/l/danube-connect-core.svg)](https://github.com/danube-messaging/danube-connect/blob/main/LICENSE)

Core SDK for building high-performance connectors for the [Danube messaging system](https://github.com/danube-messaging/danube).

## Overview

`danube-connect-core` is a production-ready framework for building connectors that integrate Danube with external systems. Whether you're building a sink connector to export messages or a source connector to import data, this SDK provides everything you need.

### Key Features

- 🔌 **Simple Trait-Based API** - Implement `SinkConnector` or `SourceConnector`
- 📦 **Batch-First Sink Runtime** - Sink connectors process `Vec<SinkRecord>` batches
- 🌊 **Polling or Streaming Sources** - Source connectors can use `poll()` or `start_streaming()`
- 📋 **Schema Registry Integration** - Automatic schema-aware serialization/deserialization
- 🔄 **Multi-Topic Routing** - Handle multiple topics with per-route dispatch and schema policy
- ⚙️ **Automatic Runtime Management** - Lifecycle, retry, metrics, and graceful shutdown
-  **Observability** - Prometheus metrics plus health-check hooks for connectors
- 🛠️ **Message Utilities** - Batching, type conversion, routing, and record context helpers
- ⚡ **Async Runtime** - Built on Tokio for efficient connector execution

## Core Concepts

### Connector Traits

**`SinkConnector`** (Danube → External System)
- Consume messages from Danube topics
- Receive batches of `SinkRecord`
- Work with typed `serde_json::Value` payloads already deserialized by the runtime
- Write to external databases, APIs, or services using `process_batch()`

**`SourceConnector`** (External System → Danube)
- Read from external systems
- Emit `SourceRecord` values wrapped in `SourceEnvelope`
- Support both polling and streaming execution modes
- Let the runtime handle schema-aware serialization before publishing

### Schema Registry Integration

**The runtime automatically handles schema operations** - your connector works with typed data:

**For Sink Connectors:**
- Runtime deserializes broker payloads using schema metadata
- `SinkRecord` exposes typed `serde_json::Value` payloads plus schema/context helpers
- You can deserialize to your domain type with `record.as_type::<T>()`

**For Source Connectors:**
- Runtime matches configured schema policy to the target topic
- Source payloads are serialized according to the configured schema type
- Schemas can be auto-registered and versioned through `SchemaMapping` / `SchemaConfig`

**Benefits:**
- ✅ Zero schema boilerplate in connector code
- ✅ Centralized schema registration and validation
- ✅ Generic record/context APIs that stay platform-agnostic
- ✅ Cached schema lookups for repeated use

### Configuration

Connectors can be configured in two ways:

**1. TOML Configuration** (recommended for standalone connectors):
```toml
danube_service_url = "http://localhost:6650"
connector_name = "my-connector"

[processing]
batch_size = 500
batch_timeout_ms = 1000

[[schemas]]
topic = "/events/users"
subject = "user-events-schema"
schema_type = "json_schema"
schema_file = "schemas/user-event.json"
version_strategy = "latest"

# Include connector-specific settings
[mqtt]
broker = "mqtt://localhost:1883"
```

**2. Programmatic Configuration** (for embedding in applications):
```rust
let config = ConnectorConfig {
    danube_service_url: "http://localhost:6650".to_string(),
    connector_name: "my-connector".to_string(),
    retry: RetrySettings::default(),
    processing: ProcessingSettings::default(),
    schemas: vec![/* schema mappings */],
};
```

For file-based loading in standalone connectors, use `ConnectorConfigLoader`.

See **[Programmatic Configuration Guide](./programmatic_config.md)** for embedding-oriented examples.

The runtime automatically:
- Loads schema definitions from files
- Registers schemas with Danube's schema registry
- Caches schemas for performance
- Handles schema versioning and evolution

### Message Types

**`SinkRecord`** - Messages from Danube
- `payload()` - Returns `&serde_json::Value` (typed data, already deserialized)
- `as_type<T>()` - Deserialize to specific Rust type
- `schema()` - Schema metadata (subject, version, type)
- `topic()`, `partition()`, `attributes()`, `publish_time()` - Message metadata
- `routing_context()` / `context()` - Generic routing and record metadata helpers

**`SourceRecord`** - Messages to Danube
- `new(topic, payload)` - Create from `serde_json::Value`
- `from_json(topic, data)` - Create from any serializable type
- `from_string(topic, text)` - Create from string
- `from_number(topic, number)` - Create from numeric value
- `from_avro(topic, data)` - Create from Avro-compatible struct
- `from_bytes(topic, data)` - Create from binary data
- `with_attribute()`, `with_key()` - Add metadata
- `routing_context()` / `context()` - Inspect generic routing metadata

**`SourceEnvelope`** - Source records plus optional checkpoint information
- `SourceEnvelope::new(record)` - Emit a record with no offset
- `SourceEnvelope::with_offset(record, offset)` - Emit a record plus checkpoint data
- Used by `SourceConnector::poll()` and `SourceSender`

### Utilities

- **`Batcher<T>`** - Message batching with size/timeout-based flushing
- **`HealthChecker`** - Health tracking with failure thresholds
- **`ConnectorMetrics`** - Prometheus metrics integration

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
danube-connect-core = "0.5"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

## Documentation

### Getting Started

- 📖 **[Connector Overview](https://danube-docs.dev-state.com/integrations/danube_connect_overview/)** - Introduction and key concepts
- 🏗️ **[Architecture Guide](https://danube-docs.dev-state.com/integrations/danube_connect_architecture/)** - Design, patterns, and schema registry
- 🛠️ **[Sink Connector Development Guide](https://danube-docs.dev-state.com/integrations/sink_connector_development/)** - Build batch-first sink connectors
- 🛠️ **[Source Connector Development Guide](https://danube-docs.dev-state.com/integrations/source_connector_development/)** - Build polling or streaming source connectors
- ⚙️ **[Programmatic Configuration](./programmatic_config.md)** - Embed runtimes in your own application

### API Reference

- 📚 **[API Documentation](https://docs.rs/danube-connect-core/latest/danube_connect_core/)** - Complete API reference on docs.rs

### Examples & Source Code

- 🔌 **[Production Connectors](https://github.com/danube-messaging/danube-connectors)** - Reference implementations:
  - MQTT Source Connector (IoT integration)
  - HTTP/Webhook Source Connector (API ingestion)
  - Qdrant Sink Connector (Vector embeddings)
  - SurrealDB Sink Connector (Multi-model database)
  - Delta Lake Sink Connector (Data lake)

- 💻 **[SDK Source Code](https://github.com/danube-messaging/danube-connect-core)** - Core framework implementation

## Performance & Best Practices

`danube-connect-core` is optimized for production workloads:

- **Async/await** - Built on Tokio for efficient async I/O
- **Schema caching** - Automatic in-memory schema lookups for efficient reuse
- **Batching** - Configurable sink batching and source publish loops
- **Runtime reuse** - Shared runtime logic for retries, metrics, and lifecycle
- **Metrics** - Built-in Prometheus metrics for monitoring and alerting
- **Context helpers** - Generic routing and record context without platform-specific coupling

See the connector development guides above for schema evolution strategies and deployment patterns.

## Contributing

Contributions are welcome! Here's how you can help:

- **Core SDK Improvements**: Enhance the connector framework, add new features, or improve performance
- **Documentation**: Improve guides, add examples, or clarify concepts
- **Bug Fixes**: Report and fix issues
- **Testing**: Add test coverage and integration tests

Please open an issue or pull request on GitHub.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](./LICENSE) for details.

## Ecosystem

- **[Danube Messaging](https://github.com/danube-messaging/danube)** - High-performance message broker
- **[Danube Connectors](https://github.com/danube-messaging/danube-connectors)** - Connectors implementations
- **[Danube Documentation](https://danube-docs.dev-state.com)** - Complete documentation portal

## Resources

- 📦 **Crates.io**: [danube-connect-core](https://crates.io/crates/danube-connect-core)
- 📚 **API Docs**: [docs.rs/danube-connect-core](https://docs.rs/danube-connect-core/latest/danube_connect_core/)
- 🐛 **Issues**: [GitHub Issues](https://github.com/danube-messaging/danube-connect-core/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/danube-messaging/danube-connect-core/discussions)
