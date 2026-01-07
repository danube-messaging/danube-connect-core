# Danube Docker Compose Setup for Connector Testing

This directory contains Docker Compose configuration for running a complete Danube cluster locally. Use this for testing all connectors in the danube-connect repository.

## Architecture Overview

The setup includes:

- **ETCD** - Metadata storage for Danube cluster coordination
- **MinIO** - S3-compatible object storage for WAL and message persistence
- **Danube Broker 1** - Primary broker (port 6650)
- **Danube Broker 2** - Secondary broker (port 6651) for HA testing

All services run in a Docker network and are fully isolated from your host machine.

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- 4GB RAM minimum (8GB recommended)
- 10GB free disk space

## Quick Start

### Step 1: Start the Cluster

```bash
cd docker
docker-compose up -d
```

### Step 2: Verify Services

```bash
docker-compose ps
```

Expected output (all services should be healthy):

```
NAME            IMAGE                                        STATUS
danube-broker1  ghcr.io/danube-messaging/danube-broker      Up (healthy)
danube-broker2  ghcr.io/danube-messaging/danube-broker      Up (healthy)
danube-etcd     quay.io/coreos/etcd:v3.5.9                  Up (healthy)
danube-minio    minio/minio:RELEASE.2025-07-23T15-54-02Z   Up (healthy)
danube-mc       minio/mc:RELEASE.2024-09-16T17-43-14Z       Up
```

### Step 3: Test Connectivity

Run a connector example:

```bash
# From the repository root
cargo run --example simple_sink
```

You should see:
```
Using default configuration for testing
SimpleSinkConnector initialized
Configuration: ConnectorConfig { ... }
```

## Service Endpoints

| Service | Endpoint | Purpose |
|---------|----------|---------|
| Danube Broker 1 | `localhost:6650` | gRPC API for producers/consumers |
| Danube Broker 2 | `localhost:6651` | Secondary broker for HA testing |
| Admin API | `localhost:50051` | Broker administration |
| Prometheus | `localhost:9040` | Metrics (broker1) / `localhost:9041` (broker2) |
| MinIO API | `localhost:9000` | S3-compatible object storage |
| MinIO Console | `localhost:9001` | MinIO web UI (user: minioadmin, pass: minioadmin123) |
| ETCD | `localhost:2379` | Metadata store |

## Testing Connectors

### Run Examples Without Configuration

All examples use sensible defaults that connect to the local cluster:

```bash
# From repository root
cargo run --example simple_sink
cargo run --example simple_source
```

### Run with Custom Configuration

Override defaults with environment variables:

```bash
DANUBE_SERVICE_URL=http://localhost:6651 \
CONNECTOR_NAME=my-sink \
DANUBE_TOPIC=/connectors/test \
cargo run --example simple_sink
```

### Test Multiple Connectors Simultaneously

Since all connectors connect to the same cluster, you can test multiple connectors in parallel:

```bash
# Terminal 1: Run sink connector
cargo run --example simple_sink

# Terminal 2: Run source connector
cargo run --example simple_source

# Terminal 3: Run another connector
cargo run --bin sink-http  # When you create it
```

All connectors will communicate through the same Danube cluster.

## Monitoring

### View Broker Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f broker1
docker-compose logs -f broker2
docker-compose logs -f etcd
docker-compose logs -f minio
```

### Check Prometheus Metrics

```bash
# Broker 1 metrics
curl http://localhost:9040/metrics

# Broker 2 metrics
curl http://localhost:9041/metrics
```

### MinIO Console

Open http://localhost:9001 in your browser:
- Username: `minioadmin`
- Password: `minioadmin123`

View uploaded WAL files and message data in the `danube-messages` bucket.

## Stopping the Cluster

### Stop All Services (Keep Data)

```bash
docker-compose stop
```

### Stop and Remove Containers (Keep Volumes)

```bash
docker-compose down
```

### Complete Reset (Delete All Data)

```bash
docker-compose down -v
```

## Configuration

### Broker Configuration

Edit `danube_broker.yml` to customize:

- **Namespaces** - Pre-created namespaces (default: `default`, `examples`, `testing`, `connectors`)
- **Auto-create topics** - Automatically create topics on first publish
- **WAL settings** - Write-ahead log configuration
- **Cloud storage** - MinIO S3 bucket and prefix
- **Policies** - Rate limits and quotas

Changes require restarting the cluster:

```bash
docker-compose down
docker-compose up -d
```

### Environment Variables

Modify `docker-compose.yml` to change:

- **RUST_LOG** - Logging level for brokers
- **MINIO credentials** - MinIO root user/password
- **Port mappings** - Expose different ports

## Troubleshooting

### Broker Won't Start

Check logs:
```bash
docker-compose logs broker1
```

Common issues:
- ETCD not healthy - wait 30 seconds and retry
- MinIO not ready - wait for `mc` service to complete
- Port already in use - change port mappings in `docker-compose.yml`

### Connection Refused

Ensure all services are healthy:
```bash
docker-compose ps
```

All services should show `Up (healthy)` or `Up`.

### Out of Disk Space

Clean up old data:
```bash
docker-compose down -v
docker system prune -a
```

### Reset Everything

```bash
docker-compose down -v
docker system prune -a
docker-compose up -d
```
