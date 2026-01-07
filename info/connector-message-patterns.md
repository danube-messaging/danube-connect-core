# Danube Connector Message Patterns & Data Flow

## Overview

This document describes common message handling patterns, data transformation strategies, and best practices for working with Danube messages in connectors.

## Understanding Danube's Message Model

### Message Structure

Danube uses a protocol buffer-based message format optimized for high-throughput streaming:

**StreamMessage fields:**
- `request_id` - Request tracking ID
- `msg_id` - Message identifier (contains producer_id, topic_name, broker_addr, topic_offset)
- `payload` - Raw binary payload
- `publish_time` - Unix timestamp (microseconds)
- `producer_name` - Producer identifier
- `subscription_name` - Subscription routing
- `attributes` - User-defined metadata (map<string, string>)

**MsgID fields:**
- `producer_id` - Producer ID within topic
- `topic_name` - Full topic name (e.g., /default/events)
- `broker_addr` - Broker address for ack routing
- `topic_offset` - Monotonic offset within topic

### Field Semantics

**Client-Populated Fields:**
- `request_id`: Unique per send operation, used for request/response correlation
- `payload`: The actual message data (schema-agnostic bytes)
- `producer_name`: Identifies the producer instance
- `attributes`: Custom key-value metadata (e.g., content-type, correlation-id)

**Broker-Populated Fields:**
- `msg_id`: Complete message identifier with routing information
- `publish_time`: Server-side timestamp for message ordering
- `subscription_name`: Added when delivering to consumers

**Key Properties:**
- `topic_offset` is monotonically increasing per partition
- `msg_id` uniquely identifies a message for acknowledgment
- `attributes` are limited to string key-value pairs (no nested structures)

## Data Flow Patterns

### Pattern 1: Sink Connector (Danube → External System)

```text
┌─────────────────────────────────────────────────────────────┐
│                        Danube Broker                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Topic: /default/orders                              │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐    │  │
│  │  │ Msg 1  │  │ Msg 2  │  │ Msg 3  │  │ Msg 4  │    │  │
│  │  └────────┘  └────────┘  └────────┘  └────────┘    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ gRPC Stream (ReceiveMessages)
              │
┌─────────────▼───────────────────────────────────────────────┐
│           Sink Connector (Consumer)                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  1. Receive StreamMessage                            │  │
│  │  2. Transform to external format                     │  │
│  │  3. Write to external system                         │  │
│  │  4. Acknowledge (Ack) back to broker                 │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ External Protocol (HTTP, SQL, etc.)
              │
┌─────────────▼───────────────────────────────────────────────┐
│                    External System                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Database / API / File System / Message Queue        │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Processing Steps:**
1. Extract payload from StreamMessage
2. Build request with metadata from attributes (topic, offset, custom headers)
3. Send request to external system (retryable operation)
4. Check response and return success/error (runtime handles ack automatically)

### Pattern 2: Source Connector (External System → Danube)

```text
┌─────────────────────────────────────────────────────────────┐
│                    External System                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  New Data: DB Changes, Log Lines, API Events         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ Poll / Stream / CDC
              │
┌─────────────▼───────────────────────────────────────────────┐
│         Source Connector (Producer)                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  1. Poll external system for new data                │  │
│  │  2. Transform to Danube SourceRecord                 │  │
│  │  3. Publish to Danube topic                          │  │
│  │  4. Commit offset/checkpoint in external system      │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ gRPC (SendMessage)
              │
┌─────────────▼───────────────────────────────────────────────┐
│                      Danube Broker                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Topic: /default/postgres-cdc                        │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐                 │  │
│  │  │ Msg 1  │  │ Msg 2  │  │ Msg 3  │  ...            │  │
│  │  └────────┘  └────────┘  └────────┘                 │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Processing Steps (PostgreSQL CDC example):**
1. Poll PostgreSQL replication stream for new changes
2. Transform each change (INSERT/UPDATE/DELETE) to SourceRecord with:
   - Payload containing operation type, table name, before/after values, LSN, timestamp
   - Attributes for source type, table name, operation type
   - Routing key based on primary key (for partitioning)
3. Publish batch to Danube topic
4. Commit LSN position in PostgreSQL after successful publish

### Pattern 3: Bridge Connector (Bidirectional)

Bridge connectors facilitate migration or integration between messaging systems:

```text
┌──────────────┐  Source Mode   ┌──────────────┐  Sink Mode    ┌──────────────┐
│   Kafka      │  ─────────────> │   Bridge     │  ────────────> │   Danube     │
│   Cluster    │                 │  Connector   │                │   Cluster    │
│              │ <───────────────│              │ <──────────────│              │
└──────────────┘   Sink Mode     └──────────────┘  Source Mode   └──────────────┘
```

**Use Cases:**
- **Migration:** Gradually move workloads from Kafka to Danube
- **Integration:** Connect Danube to existing Kafka ecosystems
- **Hybrid:** Run both systems during transition periods

## Message Transformation Patterns

### 1. Pass-Through (Minimal Transformation)

**Best for:** Binary protocols, already-serialized data

**Approach:** Direct write of payload to external system without transformation. Fastest option with minimal overhead.

### 2. JSON Transformation

**Best for:** REST APIs, document databases, analytics systems

**Approach:**
- Deserialize Danube payload from JSON
- Enrich with Danube metadata (topic, offset, timestamp, producer)
- Write enriched data to external system
- Preserves message context for downstream processing

### 3. Schema-Based Transformation

**Best for:** Strong typing requirements, Avro/Protobuf schemas

**Approach:**
- Extract schema ID from message attributes or schema registry
- Deserialize payload using schema
- Transform to target system's schema format
- Write validated data to external system
- Ensures type safety and compatibility

### 4. Batched Transformation with Aggregation

**Best for:** Analytics databases (ClickHouse, Snowflake), time-series data

**Approach:**
- Accumulate records in memory batch
- Transform each record as it arrives
- Flush batch when size or time threshold is reached:
  - Batch size limit (e.g., 1000 records)
  - Time window (e.g., 5 seconds)
- Perform bulk insert for better performance
- Clear batch and reset timer after successful write

## Error Handling Patterns

### Pattern 1: Retry with Exponential Backoff

**Use case:** Transient failures (network issues, rate limits, temporary service unavailability)

**Approach:**
- Classify errors as retryable or fatal
- Return `ConnectorError::Retryable` for temporary failures
- Runtime automatically retries with exponential backoff
- Return `ConnectorError::Fatal` for permanent errors (no retry)
- Examples of retryable: timeouts, connection errors, HTTP 429/503
- Examples of fatal: authentication errors, invalid data, HTTP 400/404

### Pattern 2: Dead Letter Queue

**Use case:** Invalid messages that should be skipped but preserved for inspection

**Approach:**
- Validate and transform incoming message
- On validation errors, send to DLQ topic
- Return success to acknowledge original message
- DLQ preserves error context for debugging
- Prevents connector from getting stuck on bad data
- Allows manual inspection and reprocessing

### Pattern 3: Partial Success in Batches

**Use case:** Batch operations where some records succeed and others fail

**Approach:**
- Perform bulk insert/write operation
- Check results for partial failures
- Collect failed records with their errors
- Retry failed records individually
- Log warnings for retries
- Ensures at-least-once delivery
- Maintains throughput while handling failures

## Performance Optimization Patterns

### 1. Connection Pooling

**Purpose:** Reuse connections to external systems, avoid overhead of creating new connections

**Configuration:**
- Set maximum pool size based on expected concurrency
- Configure connection timeout (e.g., 5 seconds)
- Use connection manager for health checks
- Initialize pool during connector startup

**Benefits:**
- Reduces connection establishment overhead
- Improves throughput for high-volume connectors
- Better resource utilization
- Handles connection failures gracefully

### 2. Async Batching with Timeout

**Purpose:** Accumulate records and flush based on size or time thresholds

**Implementation:**
- Maintain in-memory batch of records
- Non-blocking check for flush conditions
- Background timer task for time-based flushing
- Flush when batch size reaches limit OR timeout expires

**Configuration:**
- Batch size (e.g., 1000 records)
- Flush interval (e.g., 5 seconds)

**Benefits:**
- Reduces number of external system calls
- Balances latency vs throughput
- Prevents unbounded memory growth

### 3. Parallel Processing

**Purpose:** Process independent records concurrently for higher throughput

**Approach:**
- Map records to async futures
- Execute futures in parallel using join_all
- Collect and check results for failures
- Fail fast if any record fails

**Considerations:**
- Only use for independent operations (no ordering requirements)
- Manage resource consumption (limit concurrency)
- Ensure thread-safety of shared state

**Benefits:**
- Higher throughput for I/O-bound operations
- Better CPU utilization
- Reduced end-to-end latency

## Best Practices Summary

1. **Leverage Attributes:** Use message attributes for routing, tracing, and metadata
2. **Batch When Possible:** Aggregate writes for better throughput
3. **Handle Errors Gracefully:** Distinguish retryable vs fatal errors
4. **Use Structured Logging:** Include message IDs and offsets in logs
5. **Implement Health Checks:** Verify external system connectivity
6. **Monitor Performance:** Track latency, throughput, and error rates
7. **Plan for Shutdown:** Flush buffers and close connections cleanly
8. **Test with Real Data:** Validate transformation logic with production-like payloads
9. **Document Schema:** Clearly specify expected payload formats
10. **Version Attributes:** Support schema evolution via version attributes

## References

- [Danube Message Architecture](https://danube-docs.dev-state.com/architecture/messages/)
- [Connector Development Guide](./connector-development-guide.md)
