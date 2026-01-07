# Danube Connector Development Guide

## Overview

This guide explains the core concepts and requirements for building Danube connectors.

**For complete working examples:**
- [connectors/source-mqtt](../connectors/source-mqtt) - Full MQTT source implementation
- [examples/source-mqtt](../examples/source-mqtt) - Complete deployment setup

## Prerequisites

- Rust 1.70+ with async/await knowledge
- Understanding of your target external system
- Access to a Danube cluster ([docker/docker-compose.yml](../docker/docker-compose.yml) for local testing)

---

## Core Concepts

### Connector Types

**Sink Connectors** (Danube → External System)
- Consume messages from Danube topics
- Transform and write to external systems (databases, APIs, files)
- Handle batching and error recovery
- Examples: MongoDB sink, HTTP webhook, ClickHouse sink

**Source Connectors** (External System → Danube)
- Poll or listen to external systems
- Transform data into Danube messages
- Manage offsets/checkpoints
- Examples: MQTT bridge, Postgres CDC, Kafka bridge

### Architecture Layers

```
Your Connector Logic (Trait Implementation)
          ↓
danube-connect-core (Runtime & Utilities)
          ↓
danube-client (Danube Communication)
          ↓
Danube Broker Cluster
```

**Your responsibility:** Implement the trait methods  
**Core's responsibility:** Lifecycle management, retry logic, metrics, connection handling

---

## Project Setup

Create your connector project in `connectors/`:

```bash
cd connectors
cargo new --bin sink-mydb  # or source-myapi
```

### Minimal Dependencies

```toml
[dependencies]
danube-connect-core = { path = "../../danube-connect-core" }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"

# Add your external system's client library
# mongodb = "2.8"          # For MongoDB
# rumqttc = "0.24"         # For MQTT
# rdkafka = "0.36"         # For Kafka
```

---

## Configuration Architecture

### Unified Configuration Pattern

Use a **single configuration file** combining core Danube settings with connector-specific settings:

```rust
use serde::Deserialize;
use danube_connect_core::ConnectorConfig;

#[derive(Deserialize)]
pub struct MyConnectorConfig {
    #[serde(flatten)]           // Merge core config at root level
    pub core: ConnectorConfig,  // Provided by danube-connect-core
    
    pub my_connector: MySettings, // Your connector-specific settings
}
```

**Why this pattern?**
- One file for all settings (simple for users)
- Core library stays lightweight (doesn't load files)
- Clean separation of concerns
- Supports ENV overrides for secrets

**What to include in your config:**
- External system credentials (DB URL, API keys)
- Connection pool settings
- Batching parameters
- Topic mappings (for multi-topic connectors)
- Custom transformation rules

**Configuration loading pattern:**

```rust
impl MyConnectorConfig {
    pub fn load() -> ConnectorResult<Self> {
        // Load from TOML file (path from CONNECTOR_CONFIG_PATH env var)
        let config_path = env::var("CONNECTOR_CONFIG_PATH")
            .map_err(|_| ConnectorError::config(
                "CONNECTOR_CONFIG_PATH environment variable must be set"
            ))?;
        
        let mut config = Self::from_file(&config_path)?;
        
        // Apply ENV overrides (for secrets and environment-specific values)
        config.core.apply_env_overrides();
        config.my_connector.apply_env_overrides();
        
        Ok(config)
    }
    
    fn apply_env_overrides(&mut self) {
        // Core Danube settings (mandatory fields)
        if let Ok(url) = env::var("DANUBE_SERVICE_URL") {
            self.core.danube_service_url = url;
        }
        if let Ok(name) = env::var("CONNECTOR_NAME") {
            self.core.connector_name = name;
        }
        
        // Connector-specific overrides (secrets, URLs)
        // ... your connector's env var overrides
    }
}
```

**See:** `connectors/source-mqtt/src/config.rs` for complete example

---

## Implementing a Sink Connector

### The SinkConnector Trait

```rust
#[async_trait]
pub trait SinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;
    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>>;
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()>;
    
    // Optional (with defaults)
    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()>;
    async fn shutdown(&mut self) -> ConnectorResult<()>;
    async fn health_check(&self) -> ConnectorResult<()>;
}
```

### Method Responsibilities

#### 1. `initialize()` - Setup Phase
**When:** Called once at startup before any message processing

**What to do:**
- Load and validate your connector-specific configuration
- Establish connections to external systems
- Test connectivity (fail fast if misconfigured)
- Initialize internal state (connection pools, buffers)

**Why:** Ensures everything is ready before messages start flowing. Failures here stop the connector immediately.

**Example pattern:**
```rust
async fn initialize(&mut self, _config: ConnectorConfig) -> ConnectorResult<()> {
    // Config is already loaded in main.rs via MyConnectorConfig::load()
    // Just validate connector-specific settings if needed
    self.config.validate()?;
    
    // Connect to external system
    self.client = ExternalClient::connect(&self.config.url).await?;
    
    // Test connection
    self.client.ping().await?;
    
    Ok(())
}
```

---

#### 2. `consumer_configs()` - Topic Registration
**When:** Called after initialization, before message consumption starts

**What to return:**
- List of `ConsumerConfig` objects defining which Danube topics to consume from
- Each config specifies: topic name, subscription name, subscription type

**Why:** Enables multi-topic consumption. Runtime creates and manages all consumers for you.

**Example:**
```rust
async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
    Ok(vec![
        ConsumerConfig {
            topic: "/default/events".to_string(),
            consumer_name: "my-sink".to_string(),
            subscription: "my-subscription".to_string(),
            subscription_type: SubscriptionType::Exclusive,
        }
    ])
}
```

**For multi-topic:**
```rust
async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
    // Consume from multiple topics
    self.config.topics.iter().map(|topic| {
        ConsumerConfig {
            topic: topic.clone(),
            consumer_name: self.config.connector_name.clone(),
            subscription: format!("{}-sub", topic),
            subscription_type: SubscriptionType::Shared,
        }
    }).collect()
}
```

---

#### 3. `process()` - Message Handler (Required)
**When:** Called for each message received from Danube

**What to do:**
- Extract payload using helper methods
- Transform message to external system's format
- Write to external system (or buffer for batching)
- Return `Ok(())` to acknowledge, or error to retry/skip

**Error handling:**
- `Ok(())` → Message acked successfully
- `Err(ConnectorError::retryable())` → Temporary failure, runtime will retry with backoff
- `Err(ConnectorError::invalid_data())` → Bad message, skip it (logged)
- `Err(ConnectorError::fatal())` → Critical error, stop connector

**Payload extraction helpers:**
```rust
record.payload()              // &[u8] - Raw bytes
record.payload_str()?         // String - UTF-8 text
record.payload_json<T>()?     // T - Deserialize JSON
```

**Metadata access:**
```rust
record.topic()                // Topic name
record.offset()               // Message offset
record.publish_time()         // Timestamp
record.get_attribute("key")   // Custom attributes
```

**Why:** This is where your actual integration logic lives. Keep it focused on transformation and writing.

---

#### 4. `process_batch()` - Batch Optimization (Optional)
**When:** Called when runtime has collected a batch of messages

**What to do:**
- Process multiple messages in one operation
- Use bulk APIs (bulk insert, batch HTTP requests)
- More efficient than processing one-by-one

**Why:** Dramatically improves throughput for high-volume scenarios. Default implementation just calls `process()` for each record.

**Batching pattern:**
```rust
async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
    // Transform all records
    let items: Vec<_> = records.iter()
        .map(|r| self.transform(r))
        .collect::<Result<_, _>>()?;
    
    // Bulk write to external system
    self.client.bulk_insert(&items).await?;
    
    Ok(())
}
```

---

#### 5. `shutdown()` - Cleanup Phase (Optional)
**When:** Called when connector receives SIGTERM/SIGINT

**What to do:**
- Flush any buffered messages
- Close connections gracefully
- Save checkpoints if needed
- Clean up resources

**Why:** Prevents data loss and ensures graceful termination.

---

#### 6. `health_check()` - Monitoring (Optional)
**When:** Called periodically by monitoring systems

**What to do:**
- Verify connection to external system is alive
- Check that resources are accessible
- Return error if unhealthy

**Why:** Enables automated health monitoring and alerting.

---

### Common Sink Patterns

**Batching with Timeout:**
```rust
struct MySinkConnector {
    buffer: Vec<Data>,
    batch_size: usize,
    last_flush: Instant,
    flush_timeout: Duration,
}

async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    self.buffer.push(self.transform(record)?);
    
    if self.buffer.len() >= self.batch_size || 
       self.last_flush.elapsed() > self.flush_timeout {
        self.flush().await?;
        self.last_flush = Instant::now();
    }
    Ok(())
}
```

**Attribute-Based Routing:**
```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // Route based on message attributes
    let table = record.get_attribute("table").unwrap_or("default");
    let collection = self.get_collection(table);
    
    let data = record.payload_json::<MyData>()?;
    collection.insert(data).await?;
    
    Ok(())
}
```

---

## Implementing a Source Connector

### The SourceConnector Trait

```rust
#[async_trait]
pub trait SourceConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;
    async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>>;
    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>>;
    
    // Optional
    async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()>;
    async fn shutdown(&mut self) -> ConnectorResult<()>;
    async fn health_check(&self) -> ConnectorResult<()>;
}
```

### Method Responsibilities

#### 1. `initialize()` - Setup Phase
**What to do:**
- Connect to external system (MQTT broker, database, API)
- Subscribe to external topics/streams/tables
- Load last checkpoint/offset (for resumability)
- Spawn background tasks if needed (event loops, listeners)

**Why:** Prepares the connector to start receiving data from external system.

**Example:**
```rust
async fn initialize(&mut self, _config: ConnectorConfig) -> ConnectorResult<()> {
    // Connect to external system
    let client = ExternalClient::connect(&self.config.url).await?;
    
    // Subscribe to events
    client.subscribe(&self.config.source_topics).await?;
    
    // Spawn background event loop if needed
    let (tx, rx) = mpsc::channel(1000);
    tokio::spawn(event_loop(client, tx));
    self.message_rx = Some(rx);
    
    Ok(())
}
```

---

#### 2. `producer_configs()` - Declare Destinations
**When:** Called after initialization

**What to return:**
- List of all Danube topics you'll publish to
- For each topic: partition count, reliable dispatch setting

**Why:** Runtime creates all producers upfront. You just return `SourceRecord` with topic name; runtime routes it to the right producer.

**Example:**
```rust
async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>> {
    Ok(vec![
        ProducerConfig {
            topic: "/iot/sensors".to_string(),
            partitions: 8,                // Number of partitions
            reliable_dispatch: true,      // Use WAL + Cloud storage
        }
    ])
}
```

**Multi-topic example (MQTT-style):**
```rust
async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>> {
    // Create producer for each topic mapping
    self.config.topic_mappings.iter().map(|mapping| {
        ProducerConfig {
            topic: mapping.danube_topic.clone(),
            partitions: mapping.partitions,
            reliable_dispatch: mapping.reliable,
        }
    }).collect()
}
```

---

#### 3. `poll()` - Data Collection (Required)
**When:** Called repeatedly in a loop by the runtime

**What to return:**
- `Vec<SourceRecord>` with messages to publish
- Empty vec if no data available (non-blocking)

**What to do:**
- Check for new data from external system
- Transform to SourceRecord format
- Return records to publish

**Why:** This is your main data ingestion loop. Keep it non-blocking.

**Pattern:**
```rust
async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
    let mut records = Vec::new();
    
    // Check for data (non-blocking with timeout)
    match timeout(Duration::from_millis(100), self.rx.recv()).await {
        Ok(Some(data)) => {
            let record = SourceRecord::from_json("/default/events", &data)?
                .with_attribute("source", "my-connector")
                .with_key(&data.id);  // For partitioning
            
            records.push(record);
            
            // Try to get more (batch up to 100)
            while let Ok(data) = self.rx.try_recv() {
                records.push(self.transform(data)?);
                if records.len() >= 100 { break; }
            }
        }
        Ok(None) => {}, // Channel closed
        Err(_) => {},    // Timeout - no data
    }
    
    Ok(records)
}
```

**SourceRecord builders:**
```rust
SourceRecord::new(topic, payload)              // From bytes
SourceRecord::from_string(topic, text)         // From string
SourceRecord::from_json(topic, &data)          // From serializable struct
    .with_attribute(key, value)                // Add metadata
    .with_key(routing_key)                     // For partitioning
```

---

#### 4. `commit()` - Checkpoint Management (Optional)
**When:** Called after messages are successfully published and acked by Danube

**What to do:**
- Save offsets/checkpoints in external system
- Acknowledge messages if needed
- Update position markers

**Why:** Enables exactly-once or at-least-once semantics. Resume from last checkpoint after restart.

**Example:**
```rust
async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()> {
    if let Some(last_offset) = offsets.last() {
        // Save checkpoint to external system or local state
        self.checkpoint.save(last_offset).await?;
    }
    Ok(())
}
```

---

#### 5. `shutdown()` / `health_check()` - Same as Sink

---

### Common Source Patterns

**Event Loop Pattern (MQTT, Kafka):**
```rust
struct MySourceConnector {
    client: Option<ExternalClient>,
    message_rx: Option<Receiver<Data>>,
}

async fn initialize(&mut self, _config: ConnectorConfig) -> ConnectorResult<()> {
    let client = ExternalClient::connect().await?;
    let (tx, rx) = mpsc::channel(1000);
    
    // Spawn background task to receive events
    tokio::spawn(async move {
        loop {
            match client.next_event().await {
                Ok(event) => { tx.send(event).await.ok(); }
                Err(e) => { eprintln!("Error: {}", e); }
            }
        }
    });
    
    self.message_rx = Some(rx);
    Ok(())
}
```

**Polling Pattern (HTTP API, Database):**
```rust
async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
    // Query external system for new data
    let new_records = self.client
        .fetch_since(self.last_position)
        .await?;
    
    if let Some(last) = new_records.last() {
        self.last_position = last.id;
    }
    
    new_records.into_iter()
        .map(|r| SourceRecord::from_json(&self.config.topic, &r))
        .collect()
}
```

**Multi-Topic Routing:**
```rust
async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
    let external_messages = self.receive_from_external().await?;
    
    external_messages.into_iter().map(|msg| {
        // Route to different Danube topics based on external topic
        let danube_topic = self.topic_mapping.get(&msg.source_topic);
        
        SourceRecord::new(danube_topic, msg.payload)
            .with_attribute("source_topic", &msg.source_topic)
            .with_key(&msg.source_topic)  // Use as routing key
    }).collect()
}
```

---

## Main Entry Point

Your `main.rs` is minimal - just load config and start the runtime:

**Sink connector:**
```rust
#[tokio::main]
async fn main() -> ConnectorResult<()> {
    tracing_subscriber::fmt().init();
    
    let config = MyConnectorConfig::load()?;
    config.validate()?;
    
    let connector = MySinkConnector::with_config(config.my_connector);
    let mut runtime = SinkRuntime::new(connector, config.core).await?;
    
    runtime.run().await
}
```

**Source connector:**
```rust
#[tokio::main]
async fn main() -> ConnectorResult<()> {
    tracing_subscriber::fmt().init();
    
    let config = MyConnectorConfig::load()?;
    config.validate()?;
    
    let connector = MySourceConnector::with_config(config.my_connector);
    let mut runtime = SourceRuntime::new(connector, config.core).await?;
    
    runtime.run().await
}
```

**See:** `connectors/source-mqtt/src/main.rs` for complete example

---

## Error Handling Strategy

Use appropriate error types to guide runtime behavior:

```rust
// Retryable - temporary failures (network, timeouts)
ConnectorError::retryable("DB connection timeout")

// Invalid data - skip this message, continue processing
ConnectorError::invalid_data("Malformed JSON payload", payload.to_vec())

// Fatal - stop connector immediately
ConnectorError::fatal("Invalid configuration")
```

**Pattern:**
```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    let data = record.payload_json::<MyData>()
        .map_err(|e| ConnectorError::invalid_data(
            format!("Bad JSON: {}", e),
            record.payload().to_vec()
        ))?;
    
    self.client.write(data).await
        .map_err(|e| ConnectorError::retryable(
            format!("Write failed: {}", e)
        ))?;
    
    Ok(())
}
```

---

## Testing Your Connector

### Unit Tests

Test transformation logic independently:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_transformation() {
        let connector = MyConnector::new();
        let input = create_test_record();
        let output = connector.transform(&input).unwrap();
        
        assert_eq!(output.field, "expected_value");
    }
}
```

### Integration Tests

Use Docker Compose to spin up external system and Danube:

```bash
# Start test environment
docker-compose -f docker/docker-compose.yml up -d

# Run connector
cargo run

# Verify data in external system
```

**See:** `examples/source-mqtt` for complete test setup

---

## Deployment

### Dockerfile Pattern

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin my-connector

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/my-connector /usr/local/bin/
USER 1000
ENTRYPOINT ["my-connector"]
```

### Docker Compose

```yaml
services:
  my-connector:
    image: my-connector:latest
    volumes:
      - ./connector.toml:/etc/connector.toml:ro
    environment:
      # Required: Config file path
      - CONNECTOR_CONFIG_PATH=/etc/connector.toml
      
      # Optional: Core Danube overrides
      - DANUBE_SERVICE_URL=http://danube-broker:6650
      - CONNECTOR_NAME=my-connector
      
      # Optional: Connector-specific overrides (secrets, URLs)
      - MY_CONNECTOR_API_KEY=${API_KEY}
```

**See:** `examples/source-mqtt/docker-compose.yml` for complete setup

---

## Best Practices

### 1. Configuration
- Load config in `main.rs` before creating connector (via `MyConnectorConfig::load()`)
- Validate early (call `config.validate()` before starting runtime)
- Use TOML for structure, ENV vars only for secrets and environment-specific overrides
- Required ENV var: `CONNECTOR_CONFIG_PATH` (path to TOML file)
- Optional ENV overrides: `DANUBE_SERVICE_URL`, `CONNECTOR_NAME`, connector-specific secrets
- Provide sensible defaults in TOML
- Fail fast on misconfiguration

### 2. Error Handling
- Use specific error types (Retryable, InvalidData, Fatal)
- Log errors with context
- Don't panic - return errors

### 3. Performance
- Implement `process_batch()` for high throughput
- Use connection pooling
- Batch writes to external systems
- Monitor and optimize hot paths

### 4. Observability
- Use structured logging (`tracing`)
- Log important events (initialization, errors, batch flushes)
- Implement health checks
- Expose custom metrics if needed

### 5. Resource Management
- Close connections in `shutdown()`
- Flush buffers before exit
- Handle signals gracefully
- Clean up background tasks

### 6. Reliability
- Save checkpoints in `commit()`
- Handle backpressure gracefully
- Retry transient failures
- Skip invalid messages (don't crash)

---

## Resources

- **Architecture:** [info/connectors.md](./connectors.md)
- **Configuration Guide:** [info/UNIFIED_CONFIGURATION_GUIDE.md](./UNIFIED_CONFIGURATION_GUIDE.md)
- **Message Patterns:** [info/connector-message-patterns.md](./connector-message-patterns.md)
- **Complete Example:** [connectors/source-mqtt](../connectors/source-mqtt)
- **Example Setup:** [examples/source-mqtt](../examples/source-mqtt)
- **Core API:** [danube-connect-core/README.md](../danube-connect-core/README.md)

---

**Ready to start building?** Check out the [MQTT source connector](../connectors/source-mqtt) as a reference implementation!
