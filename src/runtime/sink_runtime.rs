//! Sink Runtime for Danube → External System connectors
//!
//! Handles message consumption from Danube topics and processing through sink connectors.
//! Supports multiple consumers for consuming from multiple Danube topics.

use crate::{
    ConnectorConfig, ConnectorError, ConnectorMetrics, ConnectorResult, RetryConfig, RetryStrategy,
    SinkConnector, SinkRecord, SubscriptionType,
};
use danube_client::DanubeClient;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

/// Configuration for a Danube consumer
///
/// Specifies how to create a consumer for a specific topic, including subscription settings.
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    /// Danube topic to consume from (format: /{namespace}/{topic_name})
    pub topic: String,
    /// Consumer name (for identification)
    pub consumer_name: String,
    /// Subscription name (shared across consumer instances)
    pub subscription: String,
    /// Subscription type (Exclusive, Shared, FailOver)
    pub subscription_type: SubscriptionType,
    /// Optional: Expected schema subject for validation
    /// If set, runtime will validate that incoming messages match this schema
    pub expected_schema_subject: Option<String>,
}

/// Internal struct to hold consumer and its stream together
struct ConsumerStream {
    topic: String,
    consumer: danube_client::Consumer,
    stream: mpsc::Receiver<danube_core::message::StreamMessage>,
    /// Optional expected schema subject for validation
    expected_schema_subject: Option<String>,
}

/// Runtime for Sink Connectors (Danube → External System)
///
/// Manages multiple consumers dynamically, one per source topic.
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

    /// Deserialize message payload based on schema
    ///
    /// Fetches schema from registry (with caching) and deserializes the payload
    /// into a serde_json::Value based on the schema type.
    async fn deserialize_message(
        &self,
        message: &danube_core::message::StreamMessage,
        expected_schema_subject: &Option<String>,
    ) -> ConnectorResult<(serde_json::Value, Option<crate::SchemaInfo>)> {
        use serde_json::json;
        
        if let Some(schema_id) = message.schema_id {
            // Message has schema - fetch and deserialize accordingly
            let schema = self.context.get_schema(schema_id).await?;
            
            // Validate expected schema if configured
            if let Some(expected) = expected_schema_subject {
                if &schema.subject != expected {
                    return Err(ConnectorError::invalid_data(
                        format!(
                            "Schema mismatch: expected '{}', got '{}' (schema_id: {})",
                            expected, schema.subject, schema_id
                        ),
                        Vec::new(),
                    ));
                }
            }
            
            let payload = match schema.schema_type.to_lowercase().as_str() {
                "json_schema" | "json" => {
                    serde_json::from_slice(&message.payload).map_err(|e| {
                        ConnectorError::invalid_data(
                            format!("JSON deserialization failed: {}", e),
                            message.payload.clone(),
                        )
                    })?
                },
                "string" => {
                    let s = std::str::from_utf8(&message.payload).map_err(|e| {
                        ConnectorError::invalid_data(
                            format!("UTF-8 decode failed: {}", e),
                            message.payload.clone(),
                        )
                    })?;
                    json!(s)
                },
                "number" => {
                    serde_json::from_slice(&message.payload).map_err(|e| {
                        ConnectorError::invalid_data(
                            format!("Number deserialization failed: {}", e),
                            message.payload.clone(),
                        )
                    })?
                },
                "bytes" => {
                    // Bytes - encode as base64 in JSON
                    json!({
                        "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload),
                        "size": message.payload.len()
                    })
                },
                "avro" => {
                    // TODO: Implement Avro deserialization
                    return Err(ConnectorError::config(
                        "Avro deserialization not yet implemented"
                    ));
                },
                "protobuf" => {
                    // TODO: Implement Protobuf deserialization
                    return Err(ConnectorError::config(
                        "Protobuf deserialization not yet implemented"
                    ));
                },
                _ => {
                    // Unknown type - try JSON
                    warn!("Unknown schema type '{}', attempting JSON deserialization", schema.schema_type);
                    serde_json::from_slice(&message.payload).unwrap_or_else(|_| {
                        json!({
                            "raw": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload),
                            "size": message.payload.len()
                        })
                    })
                }
            };
            
            Ok((payload, Some(schema)))
        } else {
            // No schema - try JSON, fallback to base64
            let payload = serde_json::from_slice(&message.payload).unwrap_or_else(|_| {
                debug!("Message has no schema and is not JSON, encoding as base64");
                json!({
                    "raw": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.payload),
                    "size": message.payload.len()
                })
            });
            
            Ok((payload, None))
        }
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

                        // Deserialize message with schema-aware logic
                        let (typed_payload, schema_info) = match self
                            .deserialize_message(&msg, &consumer_stream.expected_schema_subject)
                            .await
                        {
                            Ok(data) => data,
                            Err(e) => {
                                error!("Failed to deserialize message: {}", e);
                                self.metrics.record_error(&format!("{:?}", e));
                                // Skip this message and continue
                                continue;
                            }
                        };

                        // Create SinkRecord with typed payload and schema info
                        let message_id = format!(
                            "topic:{}/producer:{}/offset:{}",
                            msg.msg_id.topic_name, msg.msg_id.producer_id, msg.msg_id.topic_offset
                        );
                        
                        let record = SinkRecord {
                            payload: typed_payload,
                            attributes: msg.attributes.clone(),
                            danube_metadata: crate::message::DanubeMetadata {
                                topic: msg.msg_id.topic_name.clone(),
                                offset: msg.msg_id.topic_offset,
                                publish_time: msg.publish_time,
                                message_id,
                                producer_name: msg.producer_name.clone(),
                            },
                            partition: None, // TODO: Extract from message when partitioning is supported
                            schema_info,
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
