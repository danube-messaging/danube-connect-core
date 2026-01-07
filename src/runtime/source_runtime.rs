//! Source Runtime for External System → Danube connectors
//!
//! Handles polling external systems and publishing messages to Danube topics with
//! dynamic multi-producer management.

use crate::{
    ConnectorConfig, ConnectorError, ConnectorMetrics, ConnectorResult, SchemaConfig,
    SourceConnector, SourceRecord, VersionStrategy,
};
use danube_client::{DanubeClient, Producer};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info};

/// Runtime for Source Connectors (External System → Danube)
///
/// Manages multiple producers for publishing to Danube topics. All producers are
/// created upfront based on connector configuration.
/// Create with `SourceRuntime::new()` and run with `.run().await`.
pub struct SourceRuntime<C: SourceConnector> {
    connector: C,
    client: DanubeClient,
    producers: HashMap<String, Producer>, // topic -> producer
    config: ConnectorConfig,
    metrics: Arc<ConnectorMetrics>,
    shutdown: Arc<AtomicBool>,
    /// Schema configurations by topic (for schema registry support)
    schema_configs: HashMap<String, SchemaConfig>,
    /// Shared context with schema client and caching
    context: Arc<crate::runtime::ConnectorContext>,
}

impl<C: SourceConnector> SourceRuntime<C> {
    /// Create a new source runtime
    pub async fn new(connector: C, config: ConnectorConfig) -> ConnectorResult<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize tracing
        Self::init_tracing(&config);

        info!("Initializing Source Runtime");
        info!("Connector: {}", config.connector_name);
        info!("Danube URL: {}", config.danube_service_url);

        // Create Danube client
        let client = DanubeClient::builder()
            .service_url(&config.danube_service_url)
            .build()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to create Danube client", e))?;

        // Create metrics (topic will be set dynamically per producer)
        let metrics = Arc::new(ConnectorMetrics::new(&config.connector_name, "multi-topic"));
        metrics.set_health(true);

        // Create connector context
        let context = Arc::new(crate::runtime::ConnectorContext::new(client.clone()));

        Ok(Self {
            connector,
            client,
            producers: HashMap::new(), // Will be populated during initialization
            config,
            metrics,
            shutdown: Arc::new(AtomicBool::new(false)),
            schema_configs: HashMap::new(), // Will be populated in initialize_schemas
            context,
        })
    }

    /// Run the source connector
    pub async fn run(&mut self) -> ConnectorResult<()> {
        info!("Starting Source Runtime");

        // Setup shutdown handler
        self.setup_shutdown_handler();

        // Initialize schemas (if configured)
        self.initialize_schemas().await?;

        // Initialize connector and create producers
        self.initialize_connector().await?;
        self.create_producers().await?;

        // Main polling loop
        self.process_polling_loop().await?;

        // Graceful shutdown
        self.shutdown_connector().await?;

        Ok(())
    }

    /// Setup shutdown signal handler for SIGTERM/SIGINT
    fn setup_shutdown_handler(&self) {
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c");
            info!("Received shutdown signal");
            shutdown.store(true, Ordering::Relaxed);
        });
    }

    /// Initialize the connector
    async fn initialize_connector(&mut self) -> ConnectorResult<()> {
        info!("Initializing connector");
        self.connector.initialize(self.config.clone()).await?;
        info!("Connector initialized successfully");
        Ok(())
    }

    /// Graceful shutdown of the connector
    async fn shutdown_connector(&mut self) -> ConnectorResult<()> {
        info!("Shutting down connector");
        self.connector.shutdown().await?;
        self.metrics.set_health(false);
        info!("Source Runtime stopped");
        Ok(())
    }

    /// Initialize schema registry support
    ///
    /// Uses SchemaRegistry to load and register schemas.
    async fn initialize_schemas(&mut self) -> ConnectorResult<()> {
        let schema_registry = crate::schema::SchemaRegistry::new(self.context.clone());
        self.schema_configs = schema_registry.initialize(&self.config.schemas).await?;
        Ok(())
    }

    /// Create all producers upfront based on connector configuration
    async fn create_producers(&mut self) -> ConnectorResult<()> {
        info!("Creating producers for all configured topics");

        let producer_configs = self.connector.producer_configs().await?;

        if producer_configs.is_empty() {
            return Err(ConnectorError::config(
                "No producer configurations provided by connector",
            ));
        }

        info!("Creating {} producer(s)", producer_configs.len());

        for producer_cfg in producer_configs {
            let topic = &producer_cfg.topic;

            info!(
                "Creating producer for topic: {} (partitions: {}, reliable: {})",
                topic, producer_cfg.partitions, producer_cfg.reliable_dispatch
            );

            // Generate producer name: connector_name-topic_name
            let topic_suffix = topic.replace('/', "-");
            let producer_name = format!("{}-{}", self.config.connector_name, topic_suffix);

            let mut producer_builder = self
                .client
                .new_producer()
                .with_topic(topic)
                .with_name(&producer_name);

            // Add partitions if specified
            if producer_cfg.partitions > 0 {
                producer_builder = producer_builder.with_partitions(producer_cfg.partitions);
            }

            // Add reliable dispatch if requested
            if producer_cfg.reliable_dispatch {
                producer_builder = producer_builder.with_reliable_dispatch();
            }

            // Configure schema subject if available
            if let Some(schema_cfg) = self.schema_configs.get(topic) {
                info!(
                    "Configuring producer with schema subject: {}",
                    schema_cfg.subject
                );

                // Configure based on version strategy
                match &schema_cfg.version_strategy {
                    VersionStrategy::Latest => {
                        producer_builder =
                            producer_builder.with_schema_subject(&schema_cfg.subject);
                    }
                    VersionStrategy::Pinned(version) => {
                        producer_builder =
                            producer_builder.with_schema_version(&schema_cfg.subject, *version);
                    }
                    VersionStrategy::Minimum(min_version) => {
                        producer_builder = producer_builder
                            .with_schema_min_version(&schema_cfg.subject, *min_version);
                    }
                }
            }

            let mut producer = producer_builder.build();
            producer.create().await.map_err(|e| {
                ConnectorError::fatal_with_source(
                    format!("Failed to create producer for topic {}", topic),
                    e,
                )
            })?;

            info!("Producer created successfully for topic: {}", topic);
            self.producers.insert(topic.clone(), producer);
        }

        info!("All producers created successfully");
        Ok(())
    }

    /// Main polling loop - polls connector and publishes records
    async fn process_polling_loop(&mut self) -> ConnectorResult<()> {
        info!("Entering main polling loop");
        let poll_interval = Duration::from_millis(self.config.processing.poll_interval_ms);

        while !self.shutdown.load(Ordering::Relaxed) {
            match self.connector.poll().await {
                Ok(records) if !records.is_empty() => {
                    info!("Polled {} records", records.len());
                    self.metrics.record_batch_size(records.len());

                    // Publish records
                    match self.publish_batch(records).await {
                        Ok(offsets) => {
                            // Commit offsets
                            if let Err(e) = self.connector.commit(offsets).await {
                                error!("Failed to commit offsets: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to publish batch: {}", e);
                            self.metrics.record_error(&format!("{:?}", e));
                        }
                    }
                }
                Ok(_) => {
                    // No data, sleep briefly
                    tokio::time::sleep(poll_interval).await;
                }
                Err(e) => {
                    error!("Poll error: {}", e);
                    self.metrics.record_error(&format!("{:?}", e));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }

    /// Publish a batch of records to their respective topics
    ///
    /// Records are routed to pre-created producers based on their topic field.
    /// Each record's routing key (if present) will be used for partition selection.
    async fn publish_batch(
        &mut self,
        records: Vec<SourceRecord>,
    ) -> ConnectorResult<Vec<crate::traits::Offset>> {
        let mut offsets = Vec::new();

        for (idx, record) in records.into_iter().enumerate() {
            let start = Instant::now();
            let topic = &record.topic;

            // Serialize payload based on schema (if configured)
            let payload_bytes = if let Some(schema_cfg) = self.schema_configs.get(topic) {
                debug!("Serializing with schema type: {}", schema_cfg.schema_type);
                record.serialize_with_schema(&schema_cfg.schema_type)?
            } else {
                // No schema - serialize as JSON
                debug!("No schema configured, serializing as JSON");
                serde_json::to_vec(&record.payload).map_err(|e| {
                    ConnectorError::Serialization(format!("JSON serialization failed: {}", e))
                })?
            };

            // Get the pre-created producer for this topic
            let producer = self.producers.get_mut(topic).ok_or_else(|| {
                ConnectorError::fatal(format!(
                    "No producer found for topic: {}. Ensure producer_configs() includes this topic.",
                    topic
                ))
            })?;

            // Send message with routing key if present
            let send_result = if let Some(key) = &record.key {
                // Use key-based routing (for partitioned topics - will be used when Danube supports it)
                debug!("Sending message with key: {} to topic: {}", key, topic);
                // TODO: Use send_with_key when danube-client supports it
                // For now, key is preserved in SourceRecord but not used in actual send
                producer.send(payload_bytes, Some(record.attributes)).await
            } else {
                producer.send(payload_bytes, Some(record.attributes)).await
            };

            match send_result {
                Ok(message_id) => {
                    let duration = start.elapsed();
                    self.metrics.record_processing_time(duration);
                    self.metrics.record_success();
                    debug!("Message sent successfully: {}", message_id);

                    // Create offset
                    let offset = crate::traits::Offset::new(record.topic, idx as u64);
                    offsets.push(offset);
                }
                Err(e) => {
                    error!("Failed to publish message: {}", e);
                    return Err(ConnectorError::retryable_with_source(
                        "Failed to publish batch",
                        e,
                    ));
                }
            }
        }

        Ok(offsets)
    }

    /// Initialize tracing/logging
    fn init_tracing(config: &ConnectorConfig) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.processing.log_level));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .ok(); // Ignore if already initialized
    }
}
