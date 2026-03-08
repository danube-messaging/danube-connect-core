# Programmatic Configuration Guide

This guide covers the small part of `danube-connect-core` that is still useful when you want to embed a connector runtime directly in your own Rust application.

## Keep or delete?

This file is still useful, but only as a **short embedding guide**.

It should not duplicate the full trait and runtime documentation from the README or docs.rs. The previous version had drifted from the current API, so this file now focuses only on the programmatic setup path.

## When programmatic configuration makes sense

Use programmatic configuration when:

- you are embedding a connector runtime inside an existing Rust service
- your application already owns configuration loading and validation
- you need to build `ConnectorConfig` dynamically at runtime

Use TOML plus `ConnectorConfigLoader` when:

- you are building a standalone connector binary
- you want one file to hold both core and connector-specific settings
- you deploy with mounted config files in Docker or Kubernetes

## Building `ConnectorConfig` in code

```rust
use danube_connect_core::{
    ConnectorConfig, ProcessingSettings, RetrySettings, SchemaMapping, VersionStrategy,
};
use std::path::PathBuf;

let config = ConnectorConfig {
    danube_service_url: "http://localhost:6650".to_string(),
    connector_name: "embedded-example".to_string(),
    retry: RetrySettings {
        max_retries: 5,
        retry_backoff_ms: 1_000,
        max_backoff_ms: 30_000,
    },
    processing: ProcessingSettings {
        batch_size: 500,
        batch_timeout_ms: 1_000,
        poll_interval_ms: 100,
        metrics_port: 9090,
        log_level: "info".to_string(),
        health_check_interval_ms: 30_000,
        health_check_failure_threshold: 3,
    },
    schemas: vec![SchemaMapping {
        topic: "/events/users".to_string(),
        subject: "user-events-v1".to_string(),
        schema_type: "json_schema".to_string(),
        schema_file: PathBuf::from("schemas/user.json"),
        auto_register: true,
        version_strategy: VersionStrategy::Latest,
    }],
};
```

## Source connector example

Source connectors return `Vec<SourceEnvelope>`, not bare `SourceRecord` values.

```rust
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, Offset, ProducerConfig, SourceConnector, SourceEnvelope,
    SourceRecord,
};

struct MySourceConnector {
    cursor: u64,
}

#[async_trait]
impl SourceConnector for MySourceConnector {
    async fn initialize(&mut self, _config: ConnectorConfig) -> ConnectorResult<()> {
        Ok(())
    }

    async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>> {
        Ok(vec![ProducerConfig::new("/events/users", 0, true)])
    }

    async fn poll(&mut self) -> ConnectorResult<Vec<SourceEnvelope>> {
        let record = SourceRecord::from_json(
            "/events/users",
            serde_json::json!({ "id": self.cursor, "event": "created" }),
        )?;

        let envelope = SourceEnvelope::with_offset(
            record,
            Offset::new("users-stream", self.cursor),
        );

        self.cursor += 1;
        Ok(vec![envelope])
    }
}
```

If your source is push-based, implement `mode()` and `start_streaming()` instead of relying on polling.

## Sink connector example

Sink connectors are **batch-first** and process `Vec<SinkRecord>` through `process_batch()`.

```rust
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ConsumerConfig, SinkConnector, SinkRecord, SubscriptionType,
};

struct MySinkConnector;

#[async_trait]
impl SinkConnector for MySinkConnector {
    async fn initialize(&mut self, _config: ConnectorConfig) -> ConnectorResult<()> {
        Ok(())
    }

    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
        Ok(vec![ConsumerConfig {
            topic: "/events/users".to_string(),
            consumer_name: "embedded-sink".to_string(),
            subscription: "embedded-sink-sub".to_string(),
            subscription_type: SubscriptionType::Exclusive,
            expected_schema_subject: Some("user-events-v1".to_string()),
        }])
    }

    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
        for record in records {
            let payload = record.payload();
            let topic = record.topic();
            let _ = (payload, topic);
        }

        Ok(())
    }
}
```

## Running a runtime in your application

```rust
use danube_connect_core::{ConnectorConfig, ConnectorResult, SinkRuntime};

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    let config = ConnectorConfig::default();
    let connector = MySinkConnector;
    let mut runtime = SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
```

The same pattern works for `SourceRuntime`.

## If you want file-based config instead

For standalone binaries, prefer:

```rust
use danube_connect_core::{ConnectorConfig, ConnectorConfigLoader};

let config = ConnectorConfigLoader::new().load::<ConnectorConfig>()?;
```

That keeps connector-specific sections in TOML while still using the same runtime APIs.

## Summary

- `ConnectorConfig` can still be constructed directly in code
- `SinkConnector` uses `process_batch(Vec<SinkRecord>)`
- `SourceConnector` returns `Vec<SourceEnvelope>` or streams via `SourceSender`
- `SinkRuntime` and `SourceRuntime` are still the correct embedding entry points
