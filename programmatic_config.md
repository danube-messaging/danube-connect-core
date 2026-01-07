# Programmatic Configuration Guide

This guide explains how to use `danube-connect-core` with programmatic configuration, which is particularly useful when integrating connectors into existing Rust applications.

## When to Use Programmatic Configuration

**Use programmatic configuration when:**
- Integrating connectors into an existing application
- Building configuration dynamically at runtime
- Managing configuration through your application's own config system
- You prefer full control over configuration in code

**Use TOML configuration (`config.toml`) when:**
- Building standalone connector applications
- You need connector-specific configuration alongside core config
- You prefer declarative configuration files
- Deploying with Docker/Kubernetes where config is mounted

```rust
// Load from TOML config file (includes connector-specific config)
let config = ConnectorConfig::from_file("config.toml")?;
```

---

## Complete Configuration Structure

The `ConnectorConfig` struct contains all settings needed to run a connector:

```rust
use danube_connect_core::{
    ConnectorConfig, ProcessingSettings, RetrySettings,
    SchemaMapping, VersionStrategy
};
use std::path::PathBuf;

let config = ConnectorConfig {
    // Required: Danube broker URL
    danube_service_url: "http://localhost:6650".to_string(),
    
    // Required: Unique connector name for metrics and logging
    connector_name: "my-connector".to_string(),
    
    // Optional: Retry behavior for failed operations
    retry: RetrySettings {
        max_retries: 5,
        retry_backoff_ms: 2000,
        max_backoff_ms: 60000,
    },
    
    // Optional: Processing and performance settings
    processing: ProcessingSettings {
        batch_size: 100,
        batch_timeout_ms: 5000,
        poll_interval_ms: 100,
        metrics_port: 9090,
        log_level: "info".to_string(),
    },
    
    // Optional: Schema registry mappings for topics
    schemas: vec![
        SchemaMapping {
            topic: "/events/users".to_string(),
            subject: "user-events-v1".to_string(),
            schema_type: "json_schema".to_string(),
            schema_file: PathBuf::from("schemas/user.json"),
            version_strategy: VersionStrategy::Latest,
            auto_register: true,
        },
    ],
};
```

### Configuration Fields

#### **Required Fields**
- `danube_service_url` - Danube broker endpoint
- `connector_name` - Unique identifier for this connector instance

#### **Retry Settings**
- `max_retries` - Maximum retry attempts (default: 3)
- `retry_backoff_ms` - Initial backoff delay in milliseconds (default: 1000)
- `max_backoff_ms` - Maximum backoff delay (default: 30000)

#### **Processing Settings**
- `batch_size` - Records per batch (default: 100)
- `batch_timeout_ms` - Max wait time for batch (default: 5000)
- `poll_interval_ms` - Polling interval for source connectors (default: 100)
- `metrics_port` - Prometheus metrics port (default: 9090)
- `log_level` - Logging level: "trace", "debug", "info", "warn", "error" (default: "info")

#### **Schema Mappings**
- `topic` - Danube topic name (e.g., `/events/users`)
- `subject` - Schema registry subject name
- `schema_type` - Schema type: `json_schema`, `avro`, `string`, `number`, `bytes`
- `schema_file` - Path to schema definition file
- `version_strategy` - `Latest`, `Pinned(version)`, or `Minimum(version)`
- `auto_register` - Auto-register schema on startup (default: true)

---

## Implementing Connectors

### Source Connector (External System → Danube)

A source connector pulls data from an external system and publishes to Danube topics.

```rust
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ProducerConfig,
    SourceConnector, SourceRecord,
};

struct MySourceConnector {
    // Your connector state
}

#[async_trait]
impl SourceConnector for MySourceConnector {
    /// Initialize connector with configuration
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        // Setup your external system connection
        // Access config settings as needed
        Ok(())
    }
    
    /// Define which Danube topics to publish to
    async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>> {
        Ok(vec![
            ProducerConfig {
                topic: "/events/users".to_string(),
                partitions: 0,
                reliable_dispatch: true,
                schema_config: None, // Runtime matches from config.schemas
            }
        ])
    }
    
    /// Poll external system for new data
    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
        // Fetch data from external system
        // Create SourceRecords using helper methods:
        // - SourceRecord::from_json()
        // - SourceRecord::from_string()
        // - SourceRecord::from_number()
        // - SourceRecord::from_avro()
        // - SourceRecord::from_bytes()
        
        Ok(vec![/* your records */])
    }
    
    /// Cleanup when shutting down
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        // Close connections, cleanup resources
        Ok(())
    }
}
```

### Sink Connector (Danube → External System)

A sink connector consumes data from Danube topics and writes to an external system.

```rust
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ConsumerConfig,
    SinkConnector, SinkRecord, SubscriptionType,
};

struct MySinkConnector {
    // Your connector state
}

#[async_trait]
impl SinkConnector for MySinkConnector {
    /// Initialize connector with configuration
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        // Setup your external system connection
        Ok(())
    }
    
    /// Define which Danube topics to consume from
    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
        Ok(vec![
            ConsumerConfig {
                topic: "/events/users".to_string(),
                consumer_name: "my-sink".to_string(),
                subscription: "my-sink-sub".to_string(),
                subscription_type: SubscriptionType::Exclusive,
                expected_schema_subject: Some("user-events-v1".to_string()),
            }
        ])
    }
    
    /// Process each message from Danube
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        // Access message data
        let payload = record.payload(); // serde_json::Value
        
        // Deserialize to your type
        let data: MyDataType = record.as_type()?;
        
        // Write to external system
        
        Ok(())
    }
    
    /// Cleanup when shutting down
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        // Close connections, cleanup resources
        Ok(())
    }
}
```

---

## Creating and Running the Runtime

### Source Connector Runtime

```rust
use danube_connect_core::{ConnectorConfig, SourceRuntime, ConnectorResult};

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // 1. Create configuration programmatically
    let config = ConnectorConfig {
        danube_service_url: "http://localhost:6650".to_string(),
        connector_name: "my-source".to_string(),
        retry: RetrySettings::default(),
        processing: ProcessingSettings::default(),
        schemas: vec![/* optional schema mappings */],
    };
    
    // 2. Create your connector instance
    let connector = MySourceConnector::new();
    
    // 3. Create runtime with connector and config
    //    The runtime will:
    //    - Validate configuration
    //    - Connect to Danube broker
    //    - Initialize schema registry (if configured)
    //    - Set up metrics and logging
    //    - Call your connector's initialize() method
    let mut runtime = SourceRuntime::new(connector, config).await?;
    
    // 4. Run the connector
    //    The runtime will:
    //    - Create producers for your configured topics
    //    - Call your poll() method repeatedly
    //    - Serialize records with schema awareness
    //    - Handle retries and batching
    //    - Publish to Danube
    //    - Handle graceful shutdown on SIGTERM/SIGINT
    runtime.run().await
}
```

### Sink Connector Runtime

```rust
use danube_connect_core::{ConnectorConfig, SinkRuntime, ConnectorResult};

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // 1. Create configuration
    let config = ConnectorConfig {
        danube_service_url: "http://localhost:6650".to_string(),
        connector_name: "my-sink".to_string(),
        retry: RetrySettings::default(),
        processing: ProcessingSettings::default(),
        schemas: vec![/* optional schema mappings */],
    };
    
    // 2. Create connector instance
    let connector = MySinkConnector::new();
    
    // 3. Create runtime
    //    The runtime will:
    //    - Validate configuration
    //    - Connect to Danube broker
    //    - Initialize schema registry (if configured)
    //    - Set up retry strategy
    //    - Call your connector's initialize() method
    let mut runtime = SinkRuntime::new(connector, config).await?;
    
    // 4. Run the connector
    //    The runtime will:
    //    - Create consumers for your configured topics
    //    - Receive messages from Danube
    //    - Deserialize with schema awareness
    //    - Call your process() method for each message
    //    - Handle retries on failure
    //    - Auto-acknowledge successful processing
    //    - Handle graceful shutdown on SIGTERM/SIGINT
    runtime.run().await
}
```

---

## What the Runtime Does

The runtime handles all the heavy lifting so you can focus on your connector's business logic:

### On Startup
1. **Validates** your configuration
2. **Connects** to Danube broker
3. **Initializes** schema registry client (if schemas configured)
4. **Sets up** metrics collection (Prometheus)
5. **Configures** logging (based on `log_level`)
6. **Calls** your connector's `initialize()` method
7. **Creates** producers (source) or consumers (sink)

### During Runtime
**For Source Connectors:**
- Calls your `poll()` method at regular intervals
- Serializes records based on schema type
- Auto-registers schemas (if configured)
- Batches records for efficiency
- Publishes to Danube with retries
- Tracks metrics (messages sent, errors, latency)

**For Sink Connectors:**
- Receives messages from Danube
- Deserializes based on schema type
- Validates schema (if expected schema configured)
- Calls your `process()` method for each message
- Retries on failure (based on retry config)
- Auto-acknowledges on success
- Tracks metrics (messages processed, errors, latency)

### On Shutdown
1. **Gracefully** stops processing
2. **Calls** your connector's `shutdown()` method
3. **Flushes** any pending messages
4. **Closes** connections

---

## Alternative Configuration Methods

### From Environment Variables
```rust
use danube_connect_core::ConnectorConfig;

let config = ConnectorConfig::from_env()?;
// Reads: DANUBE_SERVICE_URL, CONNECTOR_NAME, etc.
```

### From TOML File
```rust
use danube_connect_core::ConnectorConfig;

let config = ConnectorConfig::from_file("config.toml")?;
```

**Example `config.toml`:**
```toml
danube_service_url = "http://localhost:6650"
connector_name = "my-connector"

[retry]
max_retries = 5
retry_backoff_ms = 2000

[processing]
batch_size = 100
log_level = "info"

[[schemas]]
topic = "/events/users"
subject = "user-events-v1"
schema_type = "json_schema"
schema_file = "schemas/user.json"
auto_register = true

# Your connector-specific settings
[mqtt]
broker = "mqtt://localhost:1883"
client_id = "my-connector"
```

The TOML format is particularly useful because it allows you to include **connector-specific configuration** alongside the core config.

---

## Complete Example

See `tests/config_programmatic.rs` for complete working examples of programmatic configuration.

For full connector implementations, see:
- `examples/simple_source.rs` - Basic source connector
- `examples/simple_sink.rs` - Basic sink connector
- `examples/schema_source.rs` - Schema-aware source connector
- `examples/schema_sink.rs` - Schema-aware sink connector

---

## Summary

✅ **Create `ConnectorConfig`** - Programmatically or from file  
✅ **Implement trait** - `SourceConnector` or `SinkConnector`  
✅ **Create runtime** - `SourceRuntime::new()` or `SinkRuntime::new()`  
✅ **Run** - `runtime.run().await`  

The runtime handles all infrastructure concerns (connections, schemas, retries, metrics) while you focus on your connector's core logic!
