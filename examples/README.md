# Running Examples

**Updated for Schema-First API**

The examples are designed to work out-of-the-box with sensible defaults, but can also be customized via environment variables.

## Examples Overview

### Simple Examples (No Schema Registry)

- `simple_source.rs` + `simple_sink.rs` - Work without schema registry (backward compatible)
- Runtime uses JSON serialization as fallback
- Sink must check payload type at runtime

### Schema-Aware Examples (With Schema Registry)

- `schema_source.rs` + `schema_sink.rs` - Use schema registry for type safety
- Source declares schema type (`String`) in configuration
- Runtime auto-registers schema and attaches schema_id to messages
- Sink validates schema and knows expected type (no guessing!)
- **Recommended for production use**

## Prerequisites

### Start Danube Broker

Before running any examples, you need a running Danube broker. Use the provided Docker Compose setup:

```bash
# From the project root, navigate to the docker folder
cd docker

# Start the Danube broker
docker-compose up -d

# Verify the broker is running
docker-compose ps
```

The broker will be available at `http://localhost:6650`.

To stop the broker:

```bash
cd docker
docker-compose down
```

## Quick Start (No Configuration)

**Important:** Run the **source** example first, as it creates the topic on the broker. Then run the sink example to consume the messages.

### Step 1: Simple Source Connector (Run First)

```bash
# Terminal 1 - Start the source connector
cargo run --example simple_source
```

This will:

- Connect to `http://localhost:6650` (default Danube broker)
- Create the `/default/test` topic (if it doesn't exist)
- Generate 100 test messages
- Publish them to `/default/test` topic
- Print each generated message to stdout
- Requires a running Danube broker

### Step 2: Simple Sink Connector (Run Second)

```bash
# Terminal 2 - Start the sink connector
cargo run --example simple_sink
```

This will:

- Connect to `http://localhost:6650` (default Danube broker)
- Subscribe to `/default/test` topic
- Print all received messages to stdout
- Requires a running Danube broker and the topic to exist (created by source)

## Custom Configuration (Environment Variables)

### Sink Connector with Custom Settings

```bash
DANUBE_SERVICE_URL=http://my-broker:6650 \
CONNECTOR_NAME=my-sink \
DANUBE_TOPIC=/custom/topic \
SUBSCRIPTION_NAME=my-subscription \
cargo run --example simple_sink
```

### Source Connector with Custom Settings

```bash
DANUBE_SERVICE_URL=http://my-broker:6650 \
CONNECTOR_NAME=my-source \
DANUBE_TOPIC=/custom/topic \
cargo run --example simple_source
```

## Example Output

### Simple Sink

```bash
Using default configuration for testing
To use custom settings, set environment variables:
  DANUBE_SERVICE_URL (default: http://localhost:6650)
  CONNECTOR_NAME (default: simple-sink)
  DANUBE_TOPIC (default: /default/test)
  SUBSCRIPTION_NAME (default: simple-sink-sub)

SimpleSinkConnector initialized
Configuration: ConnectorConfig { ... }
=== Message #1 ===
Topic: /default/test
Offset: 0
Producer: simple-source-simple--default-test
Publish Time: 1704579012345678
Schema: None (no schema registry used)
Payload (string): Test message #1 - Timestamp: 1704579012
Attributes:
  source = simple-source-connector
  message_number = 1
  batch_index = 0
```

**Key Changes:**

- Payload is now typed data (detects string, JSON object, array, etc.)
- Schema info is displayed (None for this simple example)
- Runtime handles all deserialization automatically

### Simple Source

```bash
Using default configuration for testing
To use custom settings, set environment variables:
  DANUBE_SERVICE_URL (default: http://localhost:6650)
  CONNECTOR_NAME (default: simple-source)
  DANUBE_TOPIC (default: /default/test)

SimpleSourceConnector initialized
Configuration: ConnectorConfig { ... }
Will generate 100 messages
Generated message #1
Generated message #2
...
```

---

## Schema-Aware Examples (Recommended)

These examples demonstrate proper schema registry integration with type safety.

### Step 1: Schema-Aware Source Connector (Run First)

```bash
# Terminal 1 - Start the schema source
cargo run --example schema_source
```

This will:

- Connect to schema registry (embedded in Danube broker)
- Auto-register "test-schema-value" subject with String type
- Generate 50 test messages with schema metadata
- Each message includes schema_id and schema_version
- No schema file needed (String type is implicit)

### Step 2: Schema-Aware Sink Connector (Run Second)

```bash
# Terminal 2 - Start the schema sink
cargo run --example schema_sink
```

This will:

- Validate incoming messages match "test-schema-value" subject
- Runtime fetches schema by ID (cached for performance)
- Deserializes payload based on schema type (String)
- **No type guessing needed** - schema tells us it's a String!

### Schema Example Output

**Schema Sink:**

```bash
=== Message #1 ===
Topic: /default/test-schema
Offset: 0
Producer: schema-source--default-test-schema
Publish Time: 1704580123456789
✓ Schema validated!
  Subject: test-schema-value
  Version: 1
  Type: string
Payload: Schema message #1 - Timestamp: 1704580123
Attributes:
  source = schema-source-connector
  message_number = 1
  batch_index = 0
```

**Key Differences from Simple Example:**

- ✓ Schema is validated (subject and version shown)
- ✓ Runtime knows type is String (no need to check `as_str()`, `as_object()`, etc.)
- ✓ Type safety enforced - mismatched schemas are rejected
- ✓ Schema registry caching for performance

---

## When to Use Which Example?

**Use Simple Examples:**

- Quick testing without schema registry
- Prototyping connectors
- Backward compatibility with systems that don't support schemas

**Use Schema-Aware Examples:**

- Production deployments
- Type safety requirements
- Working with structured data (JSON Schema, Avro, Protobuf)
- Multiple consumers need consistent schema
- Schema evolution support
