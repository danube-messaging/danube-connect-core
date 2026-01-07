//! Sink Runtime for Danube → External System connectors
//!
//! Handles message consumption from Danube topics and processing through sink connectors.
//! Supports multiple consumers for consuming from multiple Danube topics.

use crate::retry::{RetryConfig, RetryStrategy};
use crate::{
    ConnectorConfig, ConnectorError, ConnectorMetrics, ConnectorResult, SinkConnector, SinkRecord,
};
use danube_client::DanubeClient;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

/// Internal struct to hold consumer and its stream together
pub(crate) struct ConsumerStream {
    pub(crate) topic: String,
    pub(crate) consumer: danube_client::Consumer,
    pub(crate) stream: mpsc::Receiver<danube_core::message::StreamMessage>,
    /// Optional expected schema subject for validation
    pub(crate) expected_schema_subject: Option<String>,
}

/// Runtime for Sink Connectors (Danube → External System)
///
/// Manages multiple consumers dynamically, one per source topic.
/// Create with `SinkRuntime::new()` and run with `.run().await`.
pub struct SinkRuntime<C: SinkConnector> {
    connector: C,
    client: DanubeClient,
    config: ConnectorConfig,
    metrics: Arc<ConnectorMetrics>,
    retry_strategy: RetryStrategy,
    shutdown: Arc<AtomicBool>,
    /// Shared context with schema client and caching
    context: Arc<crate::runtime::ConnectorContext>,
}

impl<C: SinkConnector> SinkRuntime<C> {
    /// Create a new sink runtime
    pub async fn new(connector: C, config: ConnectorConfig) -> ConnectorResult<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize tracing
        Self::init_tracing(&config);

        info!("Initializing Sink Runtime");
        info!("Connector: {}", config.connector_name);
        info!("Danube URL: {}", config.danube_service_url);

        // Create Danube client
        let client = DanubeClient::builder()
            .service_url(&config.danube_service_url)
            .build()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to create Danube client", e))?;

        // Create metrics (topic will be set dynamically per consumer)
        let metrics = Arc::new(ConnectorMetrics::new(&config.connector_name, "multi-topic"));
        metrics.set_health(true);

        // Create retry strategy
        let retry_strategy = RetryStrategy::new(RetryConfig::new(
            config.retry.max_retries,
            config.retry.retry_backoff_ms,
            config.retry.max_backoff_ms,
        ));

        // Create connector context
        let context = Arc::new(crate::runtime::ConnectorContext::new(client.clone()));

        Ok(Self {
            connector,
            client,
            config,
            metrics,
            retry_strategy,
            shutdown: Arc::new(AtomicBool::new(false)),
            context,
        })
    }

    /// Run the sink connector with multiple consumers
    pub async fn run(&mut self) -> ConnectorResult<()> {
        info!("Starting Sink Runtime");

        // Setup shutdown handler
        self.setup_shutdown_handler();

        // Initialize connector and create consumers
        self.initialize_connector().await?;
        let mut streams = self.create_consumers().await?;

        // Main processing loop
        self.process_messages(&mut streams).await?;

        // Graceful shutdown
        self.shutdown_connector().await?;

        Ok(())
    }

    /// Setup shutdown signal handler
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

    /// Gracefully shutdown the connector
    async fn shutdown_connector(&mut self) -> ConnectorResult<()> {
        info!("Shutting down connector");
        self.connector.shutdown().await?;
        self.metrics.set_health(false);
        info!("Sink Runtime stopped");
        Ok(())
    }

    /// Create consumers for all configured topics
    ///
    /// Returns a vector of (topic, consumer, message_stream) tuples
    /// The type is intentionally opaque to avoid exposing internal danube-client stream types
    async fn create_consumers(&mut self) -> ConnectorResult<Vec<ConsumerStream>> {
        // Get consumer configurations from connector
        let consumer_configs = self.connector.consumer_configs().await?;

        if consumer_configs.is_empty() {
            return Err(ConnectorError::config(
                "No consumer configurations provided by connector",
            ));
        }

        info!("Creating {} consumer(s)", consumer_configs.len());

        let mut streams = Vec::new();

        for consumer_cfg in consumer_configs {
            info!(
                "Creating consumer for topic: {} (subscription: {}, type: {:?})",
                consumer_cfg.topic, consumer_cfg.subscription, consumer_cfg.subscription_type
            );

            let mut consumer = self
                .client
                .new_consumer()
                .with_topic(&consumer_cfg.topic)
                .with_consumer_name(&consumer_cfg.consumer_name)
                .with_subscription(&consumer_cfg.subscription)
                .with_subscription_type(consumer_cfg.subscription_type.clone().into())
                .build();

            consumer.subscribe().await.map_err(|e| {
                ConnectorError::fatal_with_source(
                    format!("Failed to subscribe to topic {}", consumer_cfg.topic),
                    e,
                )
            })?;

            let message_stream = consumer.receive().await.map_err(|e| {
                ConnectorError::fatal_with_source(
                    format!(
                        "Failed to start message stream for topic {}",
                        consumer_cfg.topic
                    ),
                    e,
                )
            })?;

            info!(
                "Consumer subscribed successfully to topic: {}",
                consumer_cfg.topic
            );

            streams.push(ConsumerStream {
                topic: consumer_cfg.topic.clone(),
                consumer,
                stream: message_stream,
                expected_schema_subject: consumer_cfg.expected_schema_subject.clone(),
            });
        }

        info!("All consumers created and subscribed successfully");
        Ok(streams)
    }


    /// Main message processing loop
    async fn process_messages(&mut self, streams: &mut [ConsumerStream]) -> ConnectorResult<()> {
        info!("Entering main processing loop");

        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            let mut has_activity = false;

            for consumer_stream in streams.iter_mut() {
                // Non-blocking check for messages (short timeout to avoid accumulation)
                match tokio::time::timeout(
                    std::time::Duration::from_millis(10),
                    consumer_stream.stream.recv(),
                )
                .await
                {
                    Ok(Some(msg)) => {
                        has_activity = true;
                        self.metrics.record_received();

                        // Convert StreamMessage to SinkRecord with schema-aware deserialization
                        let record = match SinkRecord::from_stream_message(
                            &msg,
                            &consumer_stream.expected_schema_subject,
                            &self.context,
                        )
                        .await
                        {
                            Ok(record) => record,
                            Err(e) => {
                                error!("Failed to create SinkRecord from message: {}", e);
                                self.metrics.record_error(&format!("{:?}", e));
                                // Skip this message and continue
                                continue;
                            }
                        };

                        debug!(
                            "Processing message from topic {}: offset={}, has_schema={}",
                            consumer_stream.topic,
                            record.offset(),
                            record.schema().is_some()
                        );

                        // Process with retry logic
                        match self.process_with_retry(record).await {
                            Ok(_) => {
                                // Acknowledge successful processing
                                if let Err(e) = consumer_stream.consumer.ack(&msg).await {
                                    error!("Failed to acknowledge message: {}", e);
                                } else {
                                    self.metrics.record_success();
                                    debug!("Message acknowledged");
                                }
                            }
                            Err(e) => {
                                error!("Failed to process message after retries: {}", e);
                                self.metrics.record_error(&format!("{:?}", e));
                                // TODO: Implement dead-letter queue logic
                            }
                        }
                    }
                    Ok(None) => {
                        // Channel closed
                    }
                    _ => {
                        // Timeout or no message, continue to next stream
                    }
                }
            }

            // If no activity, check for pending flushes
            if !has_activity {
                // Trigger periodic flush check for time-based flushing
                // With 10ms timeout per stream, flush checks happen every ~(N×10ms + 100ms)
                // where N is the number of consumer streams
                if let Err(e) = self.connector.process_batch(vec![]).await {
                    error!("Error during periodic flush check: {}", e);
                }

                // Sleep 100ms between check cycles to maintain reasonable flush check frequency
                // This ensures flush checks happen approximately every 100ms regardless of stream count
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Ok(())
    }

    /// Process a record with retry logic
    async fn process_with_retry(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        let start = Instant::now();
        let mut attempt = 0;

        loop {
            match self.connector.process(record.clone()).await {
                Ok(_) => {
                    let duration = start.elapsed();
                    self.metrics.record_processing_time(duration);
                    return Ok(());
                }
                Err(e) if e.is_retryable() && self.retry_strategy.should_retry(attempt) => {
                    attempt += 1;
                    self.metrics.record_retry();

                    let backoff = self.retry_strategy.calculate_backoff(attempt);
                    warn!(
                        "Retry attempt {} after {:?} - error: {}",
                        attempt, backoff, e
                    );

                    tokio::time::sleep(backoff).await;
                }
                Err(e) if e.is_invalid_data() => {
                    // Skip invalid messages
                    warn!("Skipping invalid message: {}", e);
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
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
