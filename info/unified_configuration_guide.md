# Unified Configuration Architecture

## Overview

The Danube Connect configuration system uses a **single file per deployment** pattern that combines:
- Core Danube framework settings
- Connector-specific settings

This approach provides simplicity for users while maintaining clean separation of concerns in the code.

## Philosophy

### ✅ Core Library Responsibilities (Lightweight)

The `danube-connect-core` library is intentionally minimal:
- **Provides** the `ConnectorConfig` struct
- **Validates** core configuration fields
- **Does NOT** load configuration files
- **Does NOT** manage connector-specific config

### ✅ Connector Responsibilities

Each connector binary:
- **Owns** the complete configuration loading logic
- **Defines** a unified config struct combining core + connector settings
- **Loads** from TOML file or environment variables
- **Validates** all configuration before starting

## Architecture Pattern

```
┌─────────────────────────────────────────────────────┐
│ connector.toml (Single File)                        │
│                                                      │
│ # Core settings at root level                       │
│ danube_service_url = "http://broker:6650"           │
│ connector_name = "my-connector"                     │
│                                                      │
│ [mqtt]  # or [http], [postgres], etc.               │
│ # Connector-specific settings                       │
│ broker_host = "mosquitto"                           │
│ # ...                                                │
└─────────────────────────────────────────────────────┘
                        ↓
         ┌──────────────────────────────┐
         │ ConnectorConfig (from core)  │
         │ - Lightweight struct         │
         │ - Validation only            │
         │ - No file loading            │
         └──────────────────────────────┘
                        +
         ┌──────────────────────────────┐
         │ MqttConfig (connector-owned) │
         │ - MQTT-specific fields       │
         │ - Validation logic           │
         └──────────────────────────────┘
                        ↓
         ┌──────────────────────────────┐
         │ MqttSourceConfig (unified)   │
         │ - Combines both configs      │
         │ - Handles file loading       │
         │ - Applies ENV overrides      │
         └──────────────────────────────┘
```

## Implementation Pattern

### 1. Define Unified Config Struct

```rust
// In your connector's config.rs

use danube_connect_core::{ConnectorConfig, ConnectorResult};
use serde::{Deserialize, Serialize};

/// Connector-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyConnectorConfig {
    pub target_url: String,
    pub api_key: Option<String>,
    pub batch_size: usize,
}

impl MyConnectorConfig {
    pub fn from_env() -> ConnectorResult<Self> {
        // Load from ENV vars
        Ok(Self { /* ... */ })
    }

    pub fn apply_env_overrides(&mut self) {
        // Apply ENV overrides
    }

    pub fn validate(&self) -> ConnectorResult<()> {
        // Validate settings
        Ok(())
    }
}

/// Unified configuration combining core + connector settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyConnectorUnifiedConfig {
    /// Core settings (flattened at root level of TOML)
    #[serde(flatten)]
    pub core: ConnectorConfig,

    /// Connector-specific settings (under [my_connector] section)
    pub my_connector: MyConnectorConfig,
}

impl MyConnectorUnifiedConfig {
    /// Load from single TOML file or ENV vars
    pub fn load() -> ConnectorResult<Self> {
        let mut config = if let Ok(config_file) = std::env::var("CONFIG_FILE") {
            Self::from_file(&config_file)?
        } else {
            Self::from_env()?
        };

        // Apply ENV overrides
        config.core.apply_env_overrides();
        config.my_connector.apply_env_overrides();

        Ok(config)
    }

    pub fn from_file(path: &str) -> ConnectorResult<Self> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| /* ... */)
    }

    pub fn from_env() -> ConnectorResult<Self> {
        Ok(Self {
            core: ConnectorConfig::from_env()?,
            my_connector: MyConnectorConfig::from_env()?,
        })
    }

    pub fn validate(&self) -> ConnectorResult<()> {
        self.core.validate()?;
        self.my_connector.validate()?;
        Ok(())
    }
}
```

### 2. Use in main.rs

```rust
use my_connector_config::MyConnectorUnifiedConfig;

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // Single call to load everything
    let config = MyConnectorUnifiedConfig::load()?;
    
    // Single call to validate everything
    config.validate()?;

    // Use the configs
    let connector = MyConnector::new();
    let runtime = SourceRuntime::new(connector, config.core).await?;
    runtime.run().await
}
```

### 3. Create TOML Configuration

```toml
# connector.toml - Single file for everything

# Core Danube settings at root level
danube_service_url = "http://danube-broker:6650"
connector_name = "my-connector"
destination_topic = "/my/topic"
max_retries = 3
# ... other core settings

# Connector-specific settings in named section
[my_connector]
target_url = "https://api.example.com"
api_key = "your-key"
batch_size = 100
```

## Multiple Connectors Pattern

**Question**: What if I run multiple connectors in parallel (1 source + 2-3 sinks)?

**Answer**: Each connector deployment gets its own config file.

### Deployment Pattern

```
project/
├── mqtt-source.toml         # Config for MQTT source
├── http-sink-1.toml         # Config for HTTP sink #1
├── http-sink-2.toml         # Config for HTTP sink #2
├── clickhouse-sink.toml     # Config for ClickHouse sink
└── docker-compose.yml
```

### docker-compose.yml Example

```yaml
services:
  # MQTT Source Connector
  mqtt-source:
    image: danube-source-mqtt
    volumes:
      - ./mqtt-source.toml:/etc/config.toml:ro
    environment:
      CONFIG_FILE: /etc/config.toml

  # HTTP Sink #1 - External API
  http-sink-api:
    image: danube-sink-http
    volumes:
      - ./http-sink-1.toml:/etc/config.toml:ro
    environment:
      CONFIG_FILE: /etc/config.toml
      # Override API key from secret
      HTTP_API_KEY: ${API_KEY_1}

  # HTTP Sink #2 - Webhook
  http-sink-webhook:
    image: danube-sink-http
    volumes:
      - ./http-sink-2.toml:/etc/config.toml:ro
    environment:
      CONFIG_FILE: /etc/config.toml
      HTTP_API_KEY: ${API_KEY_2}

  # ClickHouse Sink
  clickhouse-sink:
    image: danube-sink-clickhouse
    volumes:
      - ./clickhouse-sink.toml:/etc/config.toml:ro
    environment:
      CONFIG_FILE: /etc/config.toml
      CLICKHOUSE_PASSWORD: ${CH_PASSWORD}
```

### Config File Examples

**mqtt-source.toml**:
```toml
# Core settings
danube_service_url = "http://danube-broker:6650"
connector_name = "mqtt-iot-source"
destination_topic = "/iot/data"

[mqtt]
broker_host = "mosquitto"
broker_port = 1883
```

**http-sink-1.toml**:
```toml
# Core settings
danube_service_url = "http://danube-broker:6650"
connector_name = "http-api-sink"
source_topic = "/iot/data"
subscription_name = "http-api-sub"

[http]
target_url = "https://api.example.com/ingest"
method = "POST"
```

**http-sink-2.toml**:
```toml
# Core settings
danube_service_url = "http://danube-broker:6650"
connector_name = "http-webhook-sink"
source_topic = "/iot/data"
subscription_name = "webhook-sub"

[http]
target_url = "https://webhook.site/unique-id"
method = "POST"
```

## Environment Variable Overrides

ENV vars always override TOML file settings:

```bash
# Start with TOML file
export CONFIG_FILE=/etc/connector.toml

# Override specific settings
export MQTT_BROKER_HOST=production-mqtt
export DANUBE_SERVICE_URL=http://prod-broker:6650

# Secrets (don't put in TOML!)
export MQTT_PASSWORD=${MQTT_PASSWORD}
export API_KEY=${API_KEY}

# Run connector - uses TOML + overrides
./danube-source-mqtt
```
