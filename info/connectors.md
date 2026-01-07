# Danube Connect Ecosystem - Architecture & Design

## 1. Summary

The goal of **Danube Connect** is to provide a "batteries-included" ecosystem for Danube Messaging without compromising the safety, stability, or binary size of the core broker.

Instead of embedding integrations into the broker (monolithic), we will adopt a **Connect Worker Pattern**. Connectors will be standalone, memory-safe **Rust binaries** that utilize a shared core library to communicate with the Danube Cluster via the **danube-client** RPC protocol.

## 2. Design Philosophy

### 2.1 Isolation & Safety

* **No FFI in Core:** The Broker must remain pure Rust and "dumb." It should not be aware of external systems like AWS, Postgres, or Kafka.
* **Crash Isolation:** A panic in a poorly configured Postgres Connector must never crash the Message Broker. Running connectors as separate processes guarantees this isolation.
* **Process-Level Boundaries:** Each connector runs as an independent process, ensuring fault isolation and independent lifecycle management.

### 2.2 The "Shared Core" Library (`danube-connect-core`)

The `danube-connect-core` library acts as the connector SDK. Connector developers implement simple traits (`SourceConnector`, `SinkConnector`) while the core handles:

**Core Responsibilities:**

* **Danube Client Integration:** RPC communication with brokers via `danube-client`
* **Runtime Management:** Automatic lifecycle handling with `SinkRuntime` and `SourceRuntime`
* **Multi-Topic Support:** Sink connectors consume from multiple topics; source connectors publish to multiple topics
* **Message Transformation:** Helper methods for JSON, binary, and string conversions
* **Retry Logic:** Configurable exponential backoff with jitter
* **Observability:** Prometheus metrics, structured logging, and health checks
* **Configuration:** Hybrid approach - mandatory fields from env vars, optional settings from TOML
* **Error Handling:** Comprehensive error types (Fatal, Retryable, InvalidData)

## 3. Repository Structure

We will use a **separate repository** from the main broker to allow for faster iteration cycles and community contributions.

**Repository:** `github.com/danube-messaging/danube-connect`

**Workspace Layout:**

```text
/danube-connect                     (Cargo Workspace)
├── Cargo.toml                      (Workspace configuration)
├── README.md
├── LICENSE
│
├── /danube-connect-core            (The Shared SDK Library)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 (Public API exports)
│   │   ├── traits.rs              (SinkConnector, SourceConnector traits)
│   │   ├── runtime/               (SinkRuntime, SourceRuntime)
│   │   ├── message.rs             (SinkRecord, SourceRecord)
│   │   ├── config.rs              (Configuration management)
│   │   ├── error.rs               (Error types)
│   │   ├── retry.rs               (Retry strategies)
│   │   ├── metrics.rs             (Observability)
│   │   └── utils/                 (Batching, health, serialization)
│   └── examples/                   (Example implementations)
│
├── /connectors                     (Standalone Connector Binaries)
│   └── /source-mqtt                (MQTT Source - Reference Implementation)
│
├── /examples                       (Full connector examples)
│   └── /source-mqtt                (Complete MQTT example with config)
│
└── /docker                         (Testing environment)
    └── docker-compose.yml          (Local Danube cluster)
```

-----

## 4. Technical Implementation Specs

### 4.1 Communication & Message Format

Connectors communicate with Danube brokers via **gRPC** using the `danube-client` crate, which provides:

* **ProducerService:** Create producers and send messages
* **ConsumerService:** Subscribe, receive messages, and acknowledge
* **Discovery:** Topic lookup and partition information
* **HealthCheck:** Monitor client health

The `StreamMessage` contains:
* **payload** - The actual message data (bytes)
* **attributes** - Key-value metadata
* **msg_id** - Routing information (topic, offset, producer_id, broker)
* **publish_time** - Timestamp
* **producer_name** / **subscription_name** - Identity

### 4.2 Connector Trait Definitions

Located in `danube-connect-core/src/traits.rs`.

**Sink Connector (Danube → External System):**

```rust
#[async_trait]
pub trait SinkConnector: Send + Sync {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;
    
    // Returns list of topics/subscriptions to consume from
    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>>;
    
    // Process single message: Ok(()) = ack, Err(retryable) = retry, Err(fatal) = stop
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()>;
    
    // Optional: batch processing for better throughput
    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()>;
    async fn shutdown(&mut self) -> ConnectorResult<()>;
    async fn health_check(&self) -> ConnectorResult<()>;
}
```

**Source Connector (External System → Danube):**

```rust
#[async_trait]
pub trait SourceConnector: Send + Sync {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;
    
    // Returns list of destination topics and their configurations
    async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>>;
    
    // Poll for data: empty vec = no data (non-blocking)
    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>>;
    
    // Optional: commit offsets after successful publish
    async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()>;
    async fn shutdown(&mut self) -> ConnectorResult<()>;
    async fn health_check(&self) -> ConnectorResult<()>;
}
```

**Message Types:**

* **`SinkRecord`** - Wrapper around `StreamMessage` with helper methods: `payload()`, `payload_str()`, `payload_json<T>()`, `attributes()`, `topic()`, `offset()`
* **`SourceRecord`** - Data to publish with: `topic`, `payload`, `attributes`, `key` (for partitioning), optional `producer_config`

### 4.3 Message Transformation Utilities

The `danube-connect-core` provides helper methods for message transformation:

**SinkRecord helpers:**
* `payload()` - Raw bytes
* `payload_str()` - UTF-8 string
* `payload_json<T>()` - Deserialize to typed struct
* `attributes()` - Key-value metadata
* `topic()`, `offset()`, `publish_time()` - Message metadata

**SourceRecord builders:**
* `SourceRecord::new(topic, payload)` - From raw bytes
* `SourceRecord::from_string(topic, text)` - From string
* `SourceRecord::from_json(topic, &data)` - From serializable struct
* Builder pattern: `.with_attribute(k, v)`, `.with_key(key)`, `.with_producer_config(config)`

### 4.4 The Runtime Architecture

The `danube-connect-core` provides two runtime implementations:

**SinkRuntime (Danube → External):**

1. **Initialization** - Calls `connector.initialize()` and `connector.consumer_configs()`
2. **Multi-Consumer Creation** - Creates separate consumer for each configured topic
3. **Message Processing Loop** - Polls all consumers, processes messages with retry logic
4. **Error Handling** - Retryable errors trigger exponential backoff; invalid data is skipped
5. **Acknowledgment** - Successfully processed messages are acked to Danube
6. **Shutdown** - Calls `connector.shutdown()` on SIGTERM/SIGINT

**SourceRuntime (External → Danube):**

1. **Initialization** - Calls `connector.initialize()` and `connector.producer_configs()`
2. **Multi-Producer Creation** - Creates producers upfront for all configured topics
3. **Polling Loop** - Calls `connector.poll()` repeatedly at configured interval
4. **Publishing** - Routes records to appropriate producers based on `topic` field
5. **Offset Commit** - Calls `connector.commit()` after successful publish
6. **Shutdown** - Graceful cleanup on signal

**Key Features:**
* Multi-topic support (multiple consumers/producers)
* Automatic retry with exponential backoff
* Health monitoring and metrics
* Signal handling (SIGTERM, SIGINT)
* Structured logging with tracing

### 4.5 Configuration Management

**Hybrid Approach:**
* **Environment Variables** (mandatory): `DANUBE_SERVICE_URL`, `CONNECTOR_NAME`
* **TOML Files** (optional): Retry settings, processing parameters, connector-specific config
* **Connector-Managed**: Each connector loads its own specific configuration (DB credentials, endpoints, etc.)

**Example:**
```rust
// Core config from env
let config = ConnectorConfig::from_env()?;

// Or from TOML with env overrides
let mut config = ConnectorConfig::from_file("config.toml")?;
config.apply_env_overrides();
```

See [examples/source-mqtt](../examples/source-mqtt) for a complete configuration example.

### 4.6 Deployment Strategy

**Docker Deployment:**

Each connector runs as a standalone container. Minimal deployment:

```yaml
services:
  my-connector:
    image: my-connector:latest
    environment:
      DANUBE_SERVICE_URL: "http://danube-broker:6650"
      CONNECTOR_NAME: "my-connector-1"
      # Connector-specific env vars
    volumes:
      - ./config.toml:/app/config.toml  # Optional TOML config
```

**Configuration Pattern:**
* Mandatory Danube settings via env vars (portable across environments)
* Connector-specific settings via TOML or env vars
* Secrets via env vars or secret managers

For complete deployment examples, see:
* [docker/docker-compose.yml](../docker/docker-compose.yml) - Local test cluster
* [examples/source-mqtt](../examples/source-mqtt) - Full MQTT connector example with Docker setup
