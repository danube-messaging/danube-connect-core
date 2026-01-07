# Danube Connect: Connector Development Roadmap

**Last Updated:** December 2025  
**Status:** Active Development  
**Timeline:** 4 months (Q1-Q2 2026)

---

## 🎯 Strategic Vision

Position Danube as **"The Rust-Native Data Platform for AI Pipelines"** by building connectors that enable:
- **AI/RAG Pipelines** - Real-time vector embeddings for AI applications
- **Real-Time Analytics** - Feature engineering and operational analytics
- **Observability** - Complete Rust-based monitoring stack
- **Lightweight Telemetry** - Alternative to heavyweight OTel Collector

---

## ✅ Completed Connectors

### HTTP/Webhook Source Connector
**Status:** ✅ Available  
**Description:** Universal webhook ingestion from SaaS platforms  
**Documentation:** [README](../connectors/source-webhook/README.md)

### Delta Lake Sink Connector
**Status:** ✅ Available  
**Description:** ACID data lake ingestion (S3/Azure/GCS)  
**Documentation:** [README](../connectors/sink-deltalake/README.md)

---

## 📋 Implementation Order

### Phase 1: AI/Vector Ecosystem (Months 1-2)
1. [LanceDB Sink Connector](#1-lancedb-sink-connector)

### Phase 2: Observability Foundation (Month 3)
2. [OpenTelemetry Source Connector](#2-opentelemetry-source-connector)

### Phase 3: Analytics & Observability (Months 4)
3. [ClickHouse Sink Connector](#3-clickhouse-sink-connector)
4. [GreptimeDB Sink Connector](#4-greptimedb-sink-connector)

---

## 1. LanceDB Sink Connector

**Priority:** High  
**Timeline:** 3-4 weeks  
**Complexity:** Medium  
**Status:** 🚧 Planned

### Overview

A connector that streams vector embeddings from Danube topics to LanceDB, a serverless vector database that stores data in Lance format on S3. Enables RAG (Retrieval-Augmented Generation) pipelines without managing vector database infrastructure.

### Why Build This First?

#### Technical Reasons
- ✅ **Leverages Qdrant Experience** - Similar patterns to existing Qdrant sink
- ✅ **Rust-Native** - `lancedb` crate is first-class Rust support
- ✅ **Complements Qdrant** - LanceDB = disk-based, Qdrant = memory-based
- ✅ **Proven Patterns** - Builds on established sink connector architecture

#### Market Reasons
- 🎯 **RAG Pipeline Trend** - Every AI application needs vector search in 2025
- 🎯 **Serverless Positioning** - No infrastructure to manage, data lives on S3
- 🎯 **Cost Advantage** - Cheaper than Pinecone/Weaviate for large datasets
- 🎯 **AI Context Layer** - Positions Danube as "the transport for AI context"

### Use Cases

**RAG Pipeline:**
```
Documents → Embeddings API → Danube → LanceDB
(Text chunks → OpenAI embeddings → Vector search)
```

**Semantic Search:**
```
User Queries → Embedding Model → Danube → LanceDB
(Search queries → Vector embeddings → Similar results)
```

**Recommendation Engine:**
```
User Behavior → Feature Extraction → Danube → LanceDB
(Clicks/views → Behavior vectors → Similar items)
```

### Key Features

- **Vector Ingestion** - Stream embeddings with metadata
- **Multi-Table Support** - Different embedding models → different tables
- **S3-Backed Storage** - Data persists on object storage, not in memory
- **Hybrid Search** - Vector similarity + metadata filtering
- **Batch Optimization** - Configurable batch sizes for throughput
- **Schema Flexibility** - Support various embedding dimensions (384, 768, 1536, etc.)

### Technical Stack

- **Vector DB:** `lancedb` - Rust-native Lance format
- **Storage:** `object_store` - S3/Azure/GCS backends
- **Arrow:** `arrow` - Zero-copy vector operations

### Marketing Benefits

- 📣 **"Serverless Vector DB"** - No infrastructure, data on S3
- 📣 **"RAG-Ready"** - Build AI applications without Pinecone pricing
- 📣 **"Rust-Native AI Stack"** - Danube + LanceDB = all Rust
- 📣 **"Unlimited Scale"** - S3-backed storage scales infinitely
- 📣 **"Hybrid Search"** - Vector similarity + metadata filtering

### Success Metrics

- Ingest 10,000+ vectors/second
- Support embeddings up to 4096 dimensions
- Sub-50ms query latency for vector search
- Store billions of vectors on S3

---

## 2. OpenTelemetry Source Connector

**Priority:** High  
**Timeline:** 3-4 weeks  
**Complexity:** Medium  
**Status:** 🚧 Planned

### Overview

A lightweight OpenTelemetry receiver that ingests traces, metrics, and logs via OTLP (OpenTelemetry Protocol) and publishes them to Danube topics. Provides a Rust-native alternative to the heavyweight OTel Collector for teams that need simple telemetry ingestion without complex processing pipelines.

### Why Build This Second?

#### Technical Reasons
- ✅ **Builds on gRPC/HTTP Experience** - Leverages patterns from Webhook connector
- ✅ **Rust-Native Stack** - `tonic` and `prost` are mature and performant
- ✅ **Complements Observability Sinks** - Enables end-to-end observability pipeline
- ✅ **Proven Protocol** - OTLP is standardized and well-documented

#### Market Reasons
- 🎯 **OTel Collector Alternative** - 10x less memory (50MB vs 200-500MB)
- 🎯 **Simpler Deployment** - One binary instead of collector + exporters
- 🎯 **100% Rust Stack** - OTel → Danube → GreptimeDB (all Rust!)
- 🎯 **Flexible Routing** - Route telemetry to multiple backends via Danube topics
- 🎯 **Edge-Friendly** - Lightweight enough for edge deployments

### Use Cases

**Microservices Observability:**
```
Kubernetes Services (OTel SDK) → Danube OTel Source → Danube → GreptimeDB
(Traces/metrics/logs → Unified observability)
```

**Edge Telemetry:**
```
IoT Devices (OTel) → Danube OTel Source → Danube → ClickHouse
(Edge telemetry → Real-time analytics)
```

**Multi-Backend Routing:**
```
Applications → Danube OTel Source → Danube → {
  - GreptimeDB (long-term storage)
  - ClickHouse (real-time analytics)
  - Delta Lake (compliance/audit)
}
```

**Hybrid Cloud:**
```
On-Prem Services → Danube OTel Source → Danube → Cloud Backends
(Private telemetry → Secure transport → Cloud storage)
```

### Key Features

- **OTLP/gRPC** - Native OpenTelemetry protocol support
- **OTLP/HTTP** - HTTP/JSON alternative for firewall-friendly deployments
- **Multi-Signal** - Traces, metrics, and logs in one connector
- **Batching** - Configurable batch sizes for optimal throughput
- **Sampling** - Probabilistic sampling to reduce data volume
- **Metadata Enrichment** - Add service name, environment, cluster tags
- **Topic Routing** - Route different signal types to different topics
- **Compression** - gRPC compression for network efficiency

### Technical Stack

- **Protocol:** `opentelemetry-proto` - OTel protobuf definitions
- **gRPC Server:** `tonic` - High-performance gRPC framework
- **HTTP Server:** `axum` - For OTLP/HTTP endpoint
- **Serialization:** `prost` - Protobuf serialization
- **Compression:** `flate2` - gzip compression

### Configuration Example

```toml
[opentelemetry]
# gRPC endpoint for OTLP/gRPC
grpc_endpoint = "0.0.0.0:4317"

# HTTP endpoint for OTLP/HTTP
http_endpoint = "0.0.0.0:4318"

# Topic routing by signal type
traces_topic = "/observability/traces"
metrics_topic = "/observability/metrics"
logs_topic = "/observability/logs"

# Batching configuration
batch_size = 100
flush_interval_ms = 1000

# Sampling (optional)
sampling_rate = 1.0  # 1.0 = 100% (no sampling)

# Metadata enrichment
[opentelemetry.metadata]
environment = "production"
cluster = "us-west-2"
```

### Marketing Benefits

- 📣 **"Lightweight OTel Receiver"** - 10x less memory than OTel Collector
- 📣 **"Native Rust Performance"** - Sub-millisecond trace ingestion
- 📣 **"Flexible Routing"** - Route to multiple backends without exporters
- 📣 **"Edge-Ready"** - Lightweight enough for IoT and edge deployments
- 📣 **"100% Rust Stack"** - Complete observability pipeline in Rust

### Success Metrics

- Ingest 100,000+ spans/second
- Support 10,000+ metrics/second
- Sub-10ms latency from OTLP to Danube publish
- <50MB memory footprint
- 99.99% uptime

### Comparison to OTel Collector

| Feature | Danube OTel Source | OTel Collector |
|---------|-------------------|----------------|
| Memory Usage | ~50MB | 200-500MB |
| Startup Time | <1s | 10-30s |
| Language | Rust | Go |
| Routing | Danube topics | Exporters config |
| Processing | Minimal (batching) | Complex pipelines |
| Use Case | Simple ingestion | Complex transformations |

---

## 3. ClickHouse Sink Connector

**Priority:** High  
**Timeline:** 3-4 weeks  
**Complexity:** Low-Medium  
**Status:** 🚧 Planned

### Overview

A high-performance connector that streams events from Danube to ClickHouse, the fastest open-source OLAP database. Optimized for real-time analytics, feature stores, and operational dashboards.

### Why Build This Third?

#### Technical Reasons
- ✅ **Similar to SurrealDB** - Follows established sink connector patterns
- ✅ **Mature Client** - `clickhouse-rs` is production-ready
- ✅ **Async Inserts** - Leverage Rust's concurrency for ClickHouse's async_insert feature
- ✅ **Complements OTel Source** - Perfect backend for telemetry data

#### Market Reasons
- 🎯 **High Demand** - ClickHouse is exploding in popularity for real-time analytics
- 🎯 **Feature Store Use Case** - ML teams use ClickHouse for online feature serving
- 🎯 **Real-Time Dashboards** - Sub-second query latency for operational analytics
- 🎯 **Observability Backend** - Excellent for storing traces/metrics from OTel

### Use Cases

**Real-Time Feature Store:**
```
User Events → Danube → ClickHouse
(Clicks/views → Feature aggregation → ML model inference)
```

**Operational Analytics:**
```
Application Logs → Danube → ClickHouse → Grafana
(System metrics → Real-time dashboards)
```

**Time-Series Analysis:**
```
IoT Sensors → MQTT → Danube → ClickHouse
(Sensor data → Time-series queries)
```

### Key Features

- **Async Inserts** - Native ClickHouse async_insert support
- **Batch Optimization** - Configurable batch sizes and flush intervals
- **Multi-Table Routing** - Different topics → different ClickHouse tables
- **Schema Mapping** - Automatic JSON → ClickHouse schema conversion
- **Compression** - LZ4/ZSTD compression for network efficiency
- **Partitioning** - Time-based partitioning for query performance

### Technical Stack

- **ClickHouse Client:** `clickhouse` - Async Rust client
- **Compression:** `lz4` / `zstd` - Fast compression
- **Schema:** `serde` - JSON to ClickHouse type mapping

### Marketing Benefits

- 📣 **"Real-Time Analytics"** - Sub-second query latency
- 📣 **"Feature Store Ready"** - ML feature serving at scale
- 📣 **"Async Performance"** - Rust concurrency beats Java clients
- 📣 **"Cost-Effective"** - Open-source alternative to Snowflake
- 📣 **"Operational Dashboards"** - Grafana/Metabase integration

### Success Metrics

- Ingest 100,000+ rows/second
- Support 1000+ concurrent async inserts
- Sub-100ms latency from Danube to ClickHouse
- Handle billions of rows per table

---

## 4. GreptimeDB Sink Connector

**Priority:** Medium  
**Timeline:** 3-4 weeks  
**Complexity:** Low-Medium  
**Status:** 🚧 Planned

### Overview

A connector that streams time-series data from Danube to GreptimeDB, a unified observability database for metrics, logs, and traces. Written in Rust, GreptimeDB completes the "100% Rust observability stack."

### Why Build This Fourth?

#### Technical Reasons
- ✅ **Rust-to-Rust** - Both Danube and GreptimeDB are Rust-native
- ✅ **Similar to ClickHouse** - Time-series ingestion patterns
- ✅ **Emerging Technology** - Early mover advantage
- ✅ **Perfect OTel Backend** - Native support for OpenTelemetry data model

#### Market Reasons
- 🎯 **100% Rust Stack** - OTel → Danube → GreptimeDB (all Rust!)
- 🎯 **Unified Observability** - Metrics + Logs + Traces in one database
- 🎯 **IoT Positioning** - Perfect complement to MQTT source connector
- 🎯 **Less Competition** - Not as crowded as ClickHouse/Prometheus
- 🎯 **OTel Native** - First-class OpenTelemetry support

### Use Cases

**IoT Observability:**
```
MQTT Sensors → Danube → GreptimeDB → Grafana
(Sensor data → Unified metrics/logs → Dashboards)
```

**Cloud Infrastructure Monitoring:**
```
Prometheus Metrics → Danube → GreptimeDB
(System metrics → Long-term storage → Analysis)
```

**Application Observability:**
```
OpenTelemetry → Danube → GreptimeDB
(Traces/metrics/logs → Unified observability)
```

### Key Features

- **Unified Data Model** - Metrics, logs, and traces in one table
- **Time-Series Optimization** - Automatic downsampling and retention
- **PromQL Support** - Compatible with Prometheus queries
- **Multi-Tenant** - Namespace isolation for different teams
- **Compression** - Efficient time-series compression
- **S3 Tiering** - Hot/cold storage tiering

### Technical Stack

- **GreptimeDB Client:** `greptimedb` - Rust client library
- **Protocol:** gRPC for high-performance ingestion
- **Time-Series:** Native time-series data structures

### Marketing Benefits

- 📣 **"100% Rust Observability Stack"** - MQTT → Danube → GreptimeDB
- 📣 **"Unified Observability"** - Metrics + Logs + Traces
- 📣 **"IoT to Cloud"** - Complete edge-to-cloud pipeline
- 📣 **"PromQL Compatible"** - Drop-in Prometheus replacement
- 📣 **"Cost-Effective"** - Open-source, S3-backed storage

### Success Metrics

- Ingest 1M+ metrics/second
- Support 10,000+ time-series
- Sub-second query latency
- 90-day retention with automatic downsampling

---

## 🚀 Marketing Milestones

### After Connector #1 (LanceDB) - Month 2
**Positioning:** "The AI Context Transport Layer"

**Key Messages:**
- Stream embeddings to S3-backed vector DB
- Build RAG pipelines without infrastructure
- Rust-native performance for AI workloads
- Serverless vector storage
- Complete AI pipeline: Webhook → Danube → LanceDB

**Target Audience:** AI/ML teams, LLM application developers

**Demo:** Document ingestion → Embeddings → Danube → LanceDB → RAG queries

---

### After Connector #2 (OpenTelemetry) - Month 3
**Positioning:** "The Lightweight Observability Pipeline"

**Key Messages:**
- Replace OTel Collector with 10x less memory
- Native Rust performance for telemetry
- Unified platform for logs, metrics, traces
- Flexible multi-backend routing
- 100% Rust stack: OTel → Danube → GreptimeDB

**Target Audience:** DevOps teams, SRE, platform engineers

**Demo:** Microservices (OTel SDK) → Danube OTel Source → Danube → GreptimeDB → Grafana

---

### After Connector #4 (GreptimeDB) - Month 4
**Positioning:** "The Complete Rust Observability Stack"

**Key Messages:**
- 100% Rust observability stack
- IoT to cloud in one platform
- Unified metrics, logs, and traces
- Edge to cloud data pipeline
- Alternative to heavyweight OTel Collector + Prometheus

**Target Audience:** IoT companies, DevOps teams, platform engineers

**Demo:** OTel → Danube → GreptimeDB → Grafana dashboards

---

## 📊 Competitive Positioning

### vs. Kafka Connect
- ✅ **10x less memory** - Rust vs JVM
- ✅ **Faster startup** - Seconds vs minutes
- ✅ **Native Delta Lake** - No Spark required
- ✅ **Better observability** - Prometheus metrics built-in

### vs. Airbyte
- ✅ **Real-time streaming** - Not batch-based
- ✅ **Lower latency** - Sub-second vs minutes
- ✅ **Embedded deployment** - No separate infrastructure
- ✅ **Rust performance** - 10x throughput

### vs. Fivetran
- ✅ **Open-source** - No vendor lock-in
- ✅ **Self-hosted** - Data never leaves your infrastructure
- ✅ **Customizable** - Extend with Rust code
- ✅ **Cost-effective** - No per-row pricing

---

## 🎯 Success Criteria

### Technical Metrics
- **Throughput:** 100,000+ messages/second per connector
- **Latency:** Sub-100ms end-to-end
- **Memory:** <100MB per connector instance
- **Reliability:** 99.99% uptime

### Adoption Metrics
- **GitHub Stars:** 1,000+ by end of roadmap
- **Production Users:** 50+ companies
- **Community:** 500+ Discord/Slack members
- **Contributors:** 20+ external contributors

### Business Metrics
- **Enterprise Leads:** 10+ qualified leads
- **Case Studies:** 5+ published success stories
- **Conference Talks:** 3+ accepted talks
- **Blog Posts:** 20+ technical articles

---

## 📚 Resources

### Rust Crates
- **LanceDB:** `lancedb`, `arrow`
- **OpenTelemetry:** `opentelemetry-proto`, `tonic`, `prost`, `axum`
- **ClickHouse:** `clickhouse`, `lz4`, `zstd`
- **GreptimeDB:** `greptimedb`, `tonic` (gRPC)

### Documentation
- [LanceDB Rust Docs](https://lancedb.github.io/lancedb/)
- [OpenTelemetry Protocol](https://opentelemetry.io/docs/specs/otlp/)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [ClickHouse Rust Client](https://github.com/loyd/clickhouse.rs)
- [GreptimeDB Docs](https://docs.greptime.com/)

### Community
- [Danube GitHub](https://github.com/danrusei/danube)
- [Danube Connect GitHub](https://github.com/danrusei/danube-connect)

---

## 🤝 Contributing

Want to help build these connectors? See [CONTRIBUTING.md](../CONTRIBUTING.md) for:
- Development setup
- Coding standards
- Testing requirements
- Pull request process

---

**Next Steps:** Begin implementation of LanceDB Sink Connector (Target: Q1 2026)
