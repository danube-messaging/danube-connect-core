# Testing Connectors with Danube Docker Cluster

This guide explains how to test all connectors (examples and future connectors) using the shared Docker Compose setup.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Docker Compose Cluster                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Broker 1    │  │  Broker 2    │  │  ETCD        │  │
│  │ :6650       │  │ :6651       │  │ :2379       │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  MinIO (S3 Storage)                              │  │
│  │  :9000 (API) / :9001 (Console)                  │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
         ▲                    ▲                    ▲
         │                    │                    │
    ┌────┴────┐          ┌────┴────┐         ┌────┴────┐
    │ Sink    │          │ Source  │         │ Custom  │
    │Example  │          │Example  │         │Connector│
    └─────────┘          └─────────┘         └─────────┘
```

All connectors connect to the same cluster, allowing you to test:
- **Message flow** from source to sink
- **Multiple connectors** working together
- **Reliability** with broker failover
- **Persistence** with S3 storage

## Setup

### 1. Start the Docker Cluster

```bash
cd docker
docker-compose up -d
```

Wait for all services to be healthy:
```bash
docker-compose ps
```

### 2. Verify Cluster is Ready

```bash
# Check broker metrics
curl http://localhost:9040/metrics | grep danube

# Check ETCD health
curl http://localhost:2379/health

# Check MinIO
curl http://localhost:9000/minio/health/live
```

## Testing Examples

### Test 1: Run Sink Example Alone

```bash
# Terminal 1: Start sink connector
cargo run --example simple_sink

# Output:
# Using default configuration for testing
# SimpleSinkConnector initialized
# Configuration: ConnectorConfig { ... }
# (Waiting for messages...)
```

The sink will wait for messages. Leave it running.

### Test 2: Run Source Example Alone

```bash
# Terminal 2: Start source connector
cargo run --example simple_source

# Output:
# Using default configuration for testing
# SimpleSourceConnector initialized
# Configuration: ConnectorConfig { ... }
# Will generate 100 messages
# Generated message #1
# Generated message #2
# ...
```

The source generates 100 messages and publishes them to `/default/test`.

### Test 3: Run Both Together (Message Flow)

```bash
# Terminal 1: Start sink
cargo run --example simple_sink

# Terminal 2: Start source
cargo run --example simple_source
```

Expected behavior:
1. Source generates messages and publishes to `/default/test`
2. Sink subscribes to `/default/test` and receives messages
3. Sink prints each message as it arrives

Output in sink terminal:
```
=== Message #1 ===
Topic: /default/test
Offset: 0
Producer: simple-source
Publish Time: 1702400000
Payload Size: 45 bytes
Payload (text): Test message #1 - Timestamp: 1702400000
Attributes:
  source = simple-source-connector
  message_number = 1
  batch_index = 0
```

## Testing Custom Connectors


### Test the Custom Connector

```bash
# Terminal 1: Run your custom sink
cargo run --bin sink-http

# Terminal 2: Run source to generate messages
cargo run --example simple_source
```

## Testing Patterns

### Pattern 1: Source → Sink

Test message flow from source to sink:

```bash
# Terminal 1
cargo run --example simple_source

# Terminal 2
cargo run --example simple_sink
```

Verify: Sink receives all messages from source.

### Pattern 2: Multiple Sinks

Test multiple sinks consuming the same topic:

```bash
# Terminal 1: Sink 1
CONNECTOR_NAME=sink-1 SUBSCRIPTION_NAME=sink-1-sub cargo run --example simple_sink

# Terminal 2: Sink 2
CONNECTOR_NAME=sink-2 SUBSCRIPTION_NAME=sink-2-sub cargo run --example simple_sink

# Terminal 3: Source
cargo run --example simple_source
```

Verify: Both sinks receive all messages (shared subscription type).

### Pattern 3: Multiple Sources

Test multiple sources publishing to the same topic:

```bash
# Terminal 1: Source 1
CONNECTOR_NAME=source-1 cargo run --example simple_source

# Terminal 2: Source 2
CONNECTOR_NAME=source-2 cargo run --example simple_source

# Terminal 3: Sink
cargo run --example simple_sink
```

Verify: Sink receives messages from both sources.

### Pattern 4: Broker Failover

Test connector behavior when broker fails:

```bash
# Terminal 1: Run connector
cargo run --example simple_sink

# Terminal 2: Stop broker 1
docker-compose stop broker1

# Observe: Connector should attempt reconnection to broker2
# Terminal 3: Restart broker 1
docker-compose start broker1

# Observe: Connector should reconnect and resume
```

## Monitoring During Tests

### View Broker Metrics

```bash
# Real-time metrics
watch -n 1 'curl -s http://localhost:9040/metrics | grep danube_'

# Specific metrics
curl http://localhost:9040/metrics | grep -E "messages_published|messages_consumed"
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f broker1
docker-compose logs -f minio
```

### MinIO Console

Open http://localhost:9001 to view:
- Uploaded WAL files
- Message data in S3 buckets
- Storage usage

## Cleanup

### Stop Cluster (Keep Data)

```bash
docker-compose stop
```

### Remove Containers (Keep Volumes)

```bash
docker-compose down
```

### Complete Reset

```bash
docker-compose down -v
rm -rf docker/danube-data/*
```

## Troubleshooting

### Connector Can't Connect

```bash
# Check broker is running
docker-compose ps

# Check broker logs
docker-compose logs broker1

# Test connectivity
curl http://localhost:6650  # Should fail with gRPC error (expected)
```

### Messages Not Flowing

```bash
# Check topic exists
docker-compose exec broker1 danube-cli topic list

# Check subscription exists
docker-compose exec broker1 danube-cli subscription list

# Check message count
curl http://localhost:9040/metrics | grep messages_published
```

### Broker Crashed

```bash
# Check logs
docker-compose logs broker1

# Restart
docker-compose restart broker1
```

## Best Practices

1. **Start cluster first** - Always ensure Docker cluster is healthy before running connectors
2. **Use separate terminals** - Run each connector in its own terminal for easy monitoring
3. **Check logs** - Monitor both connector and broker logs during testing
4. **Test incrementally** - Start with simple examples, then test custom connectors
5. **Clean between tests** - Reset cluster data between major test runs
6. **Monitor metrics** - Watch Prometheus metrics to understand message flow

## Next Steps

1. Create your first custom connector
2. Test it with the Docker cluster
3. Add integration tests in the connector's `tests/` directory
4. Document connector-specific configuration
5. Add connector to the main repository

See `info/connector-development-guide.md` for detailed connector development instructions.
