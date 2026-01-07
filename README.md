# danube-connect-core

[![Crates.io](https://img.shields.io/crates/v/danube-connect-core.svg)](https://crates.io/crates/danube-connect-core)
[![Docs.rs](https://docs.rs/danube-connect-core/badge.svg)](https://docs.rs/danube-connect-core)
[![License](https://img.shields.io/crates/l/danube-connect-core.svg)](https://github.com/danube-messaging/danube-connect/blob/main/LICENSE)

Core SDK for building high-performance connectors for the [Danube messaging system](https://github.com/danube-messaging/danube).

## Overview

`danube-connect-core` is a production-ready framework for building connectors that integrate Danube with external systems. Whether you're building a sink connector to export messages or a source connector to import data, this SDK provides everything you need.

### Key Features

- 🔌 **Simple Trait-Based API** - Implement just `SinkConnector` or `SourceConnector` traits
- 📋 **Schema Registry Integration** - Automatic schema-aware serialization/deserialization
- 🔄 **Multi-Topic Support** - Handle multiple topics with different schemas per connector
- 🎯 **Zero Schema Boilerplate** - Runtime handles all schema operations transparently
- ⚙️ **Automatic Runtime Management** - Lifecycle, message loops, and graceful shutdown
- 🔁 **Built-in Retry Logic** - Exponential backoff with jitter for resilient integrations
- 📊 **Observability** - Prometheus metrics, structured logging, and health checks
- 🛠️ **Message Utilities** - Batching, type conversion, and validation helpers
- ⚡ **High Performance** - Async/await with Tokio, connection pooling, schema caching

## Core Concepts

### Connector Traits

**`SinkConnector`** (Danube → External System)
- Consume messages from Danube topics
- Receive typed `serde_json::Value` data (already deserialized by runtime)
- Write to external databases, APIs, or services

**`SourceConnector`** (External System → Danube)
- Read from external systems
- Provide typed `serde_json::Value` data
- Runtime handles schema-aware serialization before publishing

### Schema Registry Integration

**The runtime automatically handles schema operations** - your connector works with typed data:

**For Sink Connectors:**
- Runtime deserializes messages based on their schema
- You receive `SinkRecord` with typed `serde_json::Value` payload
- Access data directly or deserialize to structs with `as_type<T>()`
- Schema info available via `record.schema()`

**For Source Connectors:**
- Create `SourceRecord` with typed `serde_json::Value` data
- Runtime serializes based on topic's configured schema
- Automatic schema registration and validation
- Support for JSON Schema, String, Bytes, Number (Avro & Protobuf coming soon)

**Benefits:**
- ✅ Zero schema boilerplate in connector code
- ✅ Type safety and data validation
- ✅ Managed schema evolution with version strategies
- ✅ Automatic caching for performance
- ✅ Centralized schema management

### Runtime Management

Both `SinkRuntime` and `SourceRuntime` handle:
- Schema registry client initialization and caching
- Schema-aware serialization/deserialization
- Message polling and processing loops
- Automatic retry with exponential backoff
- Graceful shutdown and signal handling
- Prometheus metrics and health checks

### Configuration

Connectors use TOML configuration files with environment variable overrides:

**Core settings:**
- Danube broker URL and connector name
- Retry and processing behavior
- Metrics and logging configuration

**Schema registry configuration:**
```toml
[[schemas]]
topic = "/events/users"
subject = "user-events-schema"
schema_type = "json_schema"
schema_file = "schemas/user-event.json"
version_strategy = "latest"  # or "pinned" or "minimum"
```

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
- `topic()`, `offset()`, `attributes()`, `publish_time()` - Message metadata

**`SourceRecord`** - Messages to Danube
- `new(topic, payload)` - Create from `serde_json::Value`
- `from_json(topic, data)` - Create from any serializable type
- `from_string(topic, text)` - Create from string
- `with_attribute()`, `with_key()` - Add metadata

### Utilities

- **`Batcher<T>`** - Message batching with size/timeout-based flushing
- **`HealthChecker`** - Health tracking with failure thresholds
- **`ConnectorMetrics`** - Prometheus metrics integration

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
danube-connect-core = "0.3"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

## Documentation

### Getting Started

- 📖 **[Connector Overview](https://danube-docs.dev-state.com/integrations/danube_connect_overview/)** - Introduction and key concepts
- 🏗️ **[Architecture Guide](https://danube-docs.dev-state.com/integrations/danube_connect_architecture/)** - Design, patterns, and schema registry
- 🛠️ **[Development Guide](https://danube-docs.dev-state.com/integrations/danube_connect_development/)** - Build your own connector with schema registry

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
- **Schema caching** - Automatic in-memory schema cache with LRU eviction
- **Batching** - Process messages in configurable batches for higher throughput
- **Connection pooling** - Reuse connections to external systems and Danube
- **Metrics** - Built-in Prometheus metrics for monitoring and alerting
- **Health checks** - HTTP endpoint for Kubernetes liveness/readiness probes

See the [Development Guide](https://danube-docs.dev-state.com/integrations/danube_connect_development/) for schema evolution strategies and production deployment patterns.


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
