# Danube Connect - Comprehensive Documentation

This directory contains the complete architectural design and documentation for the **Danube Connect** ecosystem - a connector framework for integrating Danube Messaging with external systems.

## 📚 Documentation Index

### 1. [connectors.md](./connectors.md) - Architecture & Design Document
**High-level architecture and design philosophy**

- Summary and design philosophy
- Repository structure and workspace layout
- Technical implementation specifications
- Connector traits and message types
- Runtime architecture (SinkRuntime, SourceRuntime)
- Configuration management and deployment strategies

**Key Topics:**
- Process isolation and safety guarantees
- Shared core library responsibilities
- Trait-based connector design
- Multi-topic support
- Hybrid configuration approach (env vars + TOML)

### 2. [connector-development-guide.md](./connector-development-guide.md) - Developer Guide
**Conceptual guide explaining what to implement and why**

- Core concepts (connector types, architecture layers)
- Project setup and minimal dependencies
- Configuration architecture (unified pattern)
- Trait method responsibilities and patterns
- Error handling strategies
- Testing and deployment approaches

**Key Topics:**
- Understanding `SinkConnector` and `SourceConnector` traits
- What each trait method does and when it's called
- Common patterns (batching, multi-topic, error handling)
- Best practices for reliability and performance
- References to complete examples (source-mqtt)

### 3. [connector-message-patterns.md](./connector-message-patterns.md) - Message Handling Guide
**Deep dive into message transformation and data flow**

- Danube message structure and semantics
- Data flow patterns (sink, source, bridge)
- Message transformation strategies (pass-through, JSON, schema-based, batched)
- Using message attributes for routing and metadata
- Error handling patterns (retry, DLQ, partial success)
- Performance optimization techniques

**Key Topics:**
- Understanding `StreamMessage` and `MsgID`
- Field ownership (client vs broker)
- Transformation patterns
- Attribute-based routing
- Batch processing optimization

### 4. [unified_configuration_guide.md](./unified_configuration_guide.md) - Configuration Patterns
**For understanding the configuration architecture**

- Single-file configuration pattern
- TOML-first approach with ENV overrides
- Core vs connector-specific settings
- Multiple connector deployment patterns
- Docker and Kubernetes examples

**Key Topics:**
- Unified config struct design
- Environment variable overrides
- Multi-connector deployments
- Configuration best practices


## 🏗️ Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────┐
│                    External Systems                             │
│  (PostgreSQL, HTTP APIs, ClickHouse, MQTT, Kafka, etc.)        │
└──────────────────┬──────────────────────────────────────────────┘
                   │ External Protocols
┌──────────────────▼──────────────────────────────────────────────┐
│               Connector Implementations                         │
│  (sink-http, source-postgres, sink-clickhouse, etc.)           │
│  - Implements SinkConnector or SourceConnector traits          │
│  - Business logic for external system integration              │
└──────────────────┬──────────────────────────────────────────────┘
                   │ Uses Connector SDK
┌──────────────────▼──────────────────────────────────────────────┐
│                danube-connect-core                              │
│  - Runtime & lifecycle management                              │
│  - Message transformation utilities                            │
│  - Retry & error handling                                      │
│  - Configuration & observability                               │
└──────────────────┬──────────────────────────────────────────────┘
                   │ Wraps & Manages
┌──────────────────▼──────────────────────────────────────────────┐
│                   danube-client                                 │
│  - Producer/Consumer high-level API                            │
│  - gRPC client implementation                                  │
│  - Connection management & health checks                       │
└──────────────────┬──────────────────────────────────────────────┘
                   │ gRPC/Protobuf (DanubeApi.proto)
┌──────────────────▼──────────────────────────────────────────────┐
│                  Danube Broker Cluster                          │
│  - Topic management & message routing                          │
│  - WAL + Cloud persistence                                     │
│  - Subscription management                                     │
└─────────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### For Connector Users

1. **Choose a connector** from the available implementations
2. **Configure via environment variables**:
   ```bash
   export DANUBE_SERVICE_URL="http://localhost:6650"
   export DANUBE_TOPIC="/default/events"
   export SUBSCRIPTION_NAME="my-sink"
   ```
3. **Run with Docker**:
   ```bash
   docker run -e DANUBE_SERVICE_URL=... danube-connect/sink-http:latest
   ```

### For Connector Developers

1. **Read** [connector-development-guide.md](./connector-development-guide.md)
2. **Create** a new connector project using the template
3. **Implement** the `SinkConnector` or `SourceConnector` trait
4. **Test** with your local Danube cluster
5. **Contribute** back to the community!

## 🎯 Key Design Principles

### 1. **Isolation & Safety**
- Connectors run as separate processes
- No FFI or unsafe code in core broker
- Crash in connector doesn't affect broker

### 2. **Minimal Interface**
- Developers implement 2-3 trait methods
- SDK handles all infrastructure concerns
- Focus on business logic, not plumbing

### 3. **Performance First**
- Built-in batching support
- Connection pooling
- Parallel processing capabilities
- Zero-copy where possible

### 4. **Observability Built-in**
- Prometheus metrics out of the box
- Structured logging with tracing
- Health check endpoints
- Automatic error tracking

### 5. **Cloud Native**
- 12-factor app configuration
- Docker-first deployment
- Horizontal scalability
- Kubernetes-ready

## 🛠️ Development Workflow

```text
1. Design Phase
   ├── Define external system integration requirements
   ├── Choose connector type (Sink/Source/Bridge)
   └── Plan message transformation strategy

2. Implementation Phase
   ├── Create connector project structure
   ├── Implement connector trait
   ├── Add configuration handling
   ├── Implement transformation logic
   └── Add error handling

3. Testing Phase
   ├── Unit tests for transformation logic
   ├── Integration tests with real external system
   ├── Performance testing with load
   └── Failure scenario testing

4. Deployment Phase
   ├── Create Dockerfile
   ├── Build Docker image
   ├── Deploy with docker-compose or Kubernetes
   └── Monitor metrics and logs

5. Maintenance Phase
   ├── Monitor connector health
   ├── Update dependencies
   ├── Add features based on feedback
   └── Performance optimization
```

## 🔧 Configuration Reference

### Common Environment Variables

```bash
# Danube connection
DANUBE_SERVICE_URL="http://localhost:6650"
CONNECTOR_NAME="my-connector-1"

# For Sink Connectors
DANUBE_TOPIC="/default/source-topic"
SUBSCRIPTION_NAME="my-subscription"
SUBSCRIPTION_TYPE="Exclusive"  # or Shared, Failover

# For Source Connectors  
DANUBE_TOPIC="/default/destination-topic"
DISPATCH_STRATEGY="Reliable"  # or NonReliable

# Runtime configuration
MAX_RETRIES=3
RETRY_BACKOFF_MS=1000
BATCH_SIZE=1000
BATCH_TIMEOUT_MS=1000
POLL_INTERVAL_MS=100

# Observability
METRICS_PORT=9090
LOG_LEVEL="info"
RUST_LOG="info,danube_connect_core=debug"
```

## 🤝 Contributing

We welcome contributions! Areas where we need help:

- **New Connectors:** Implement connectors for popular systems
- **Documentation:** Improve guides and examples
- **Testing:** Add test coverage and integration tests
- **Performance:** Optimize hot paths and reduce overhead
- **Features:** Dead-letter queues, schema evolution, exactly-once semantics

## 📞 Support & Community

- **Documentation:** You're reading it!
- **GitHub Issues:** Report bugs and request features
- **Discord/Slack:** Join the Danube community (link TBD)
- **Email:** support@danube-messaging.io (if applicable)

## 📄 License

Danube Connect follows the same license as Danube Messaging (check main repository).

---

**Ready to build your first connector?** Start with the [connector-development-guide.md](./connector-development-guide.md)!
