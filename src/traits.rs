//! Connector trait definitions.

use crate::{
    ConnectorConfig, ConnectorError, ConnectorResult, ConsumerConfig, ProducerConfig, SinkRecord,
    SourceRecord,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Trait for implementing Sink Connectors (Danube → External System)
///
/// Sink connectors consume messages from Danube topics and write them to external systems.
///
/// # Example
///
/// ```rust,no_run
/// use danube_connect_core::{SinkConnector, SinkRecord, ConnectorConfig, ConnectorResult, ConsumerConfig, SubscriptionType};
/// use async_trait::async_trait;
/// use std::env;
///
/// pub struct HttpSink {
///     target_url: String,
/// }
///
/// #[async_trait]
/// impl SinkConnector for HttpSink {
///     async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
///         // Connectors now manage their own config
///         self.target_url = env::var("TARGET_URL")
///             .map_err(|_| danube_connect_core::ConnectorError::config("TARGET_URL required"))?;
///         Ok(())
///     }
///     
///     async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
///         Ok(vec![ConsumerConfig {
///             topic: "/default/http-topic".to_string(),
///             consumer_name: "http-sink-consumer".to_string(),
///             subscription: "http-sink-sub".to_string(),
///             subscription_type: SubscriptionType::Exclusive,
///             expected_schema_subject: None,
///         }])
///     }
///     
///     async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
///         // Send a batch to the external system
///         for record in records {
///             let _ = record;
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait SinkConnector: Send + Sync {
    /// Initialize the connector with configuration
    ///
    /// This method is called once at startup before any message processing begins.
    /// Use it to:
    /// - Load connector-specific configuration
    /// - Establish connections to external systems
    /// - Validate credentials and connectivity
    /// - Initialize internal state
    ///
    /// # Errors
    ///
    /// Return `ConnectorError::Configuration` for configuration issues
    /// Return `ConnectorError::Fatal` for initialization failures
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;

    /// Get consumer configurations for topics to consume from
    ///
    /// This method should return a list of consumer configurations, one for each
    /// Danube topic the connector wants to consume from. The runtime will create
    /// and manage these consumers automatically.
    ///
    /// # Returns
    ///
    /// A vector of `ConsumerConfig` specifying which topics to consume from and
    /// their subscription settings.
    ///
    /// # Note
    ///
    /// This method is called after `initialize()` and before message processing begins.
    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>>;

    /// Process a batch of messages from Danube
    ///
    /// This method is called by the runtime whenever the configured batch size or
    /// batch timeout is reached. Connectors should treat this as the primary sink
    /// write path.
    ///
    /// # Return Value
    ///
    /// - `Ok(())`: Batch processed successfully, messages will be acknowledged
    /// - `Err(ConnectorError::Retryable)`: Transient failure, batch will be retried
    /// - `Err(ConnectorError::Fatal)`: Permanent failure, connector will stop
    /// - `Err(ConnectorError::InvalidData)`: Invalid batch content or message data
    ///
    /// # Errors
    ///
    /// Return appropriate error type based on the failure scenario
    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()>;

    /// Optional: Called before shutdown for cleanup
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use danube_connect_core::{SinkConnector, SinkRecord, ConnectorConfig, ConnectorResult, ConsumerConfig, SubscriptionType};
    /// # use async_trait::async_trait;
    /// # struct MyConnector;
    /// # #[async_trait]
    /// # impl SinkConnector for MyConnector {
    /// #     async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> { Ok(()) }
    /// #     async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
    /// #         Ok(vec![ConsumerConfig {
    /// #             topic: "/default/test".to_string(),
    /// #             consumer_name: "test-consumer".to_string(),
    /// #             subscription: "test-sub".to_string(),
    /// #             subscription_type: SubscriptionType::Exclusive,
    /// #             expected_schema_subject: None,
    /// #         }])
    /// #     }
    /// async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
    ///     // Bulk insert all records in one operation
    ///     // self.database.bulk_insert(&records).await?;
    ///     let _ = records;
    ///     Ok(())
    /// }
    /// # }
    /// ```
    /// Use this to:
    /// - Flush any pending writes
    /// - Close connections gracefully
    /// - Save checkpoints
    /// - Clean up resources
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        Ok(())
    }

    /// Optional: Health check implementation
    /// This method is called periodically to verify the connector is healthy.
    /// Check connectivity to external systems and return an error if unhealthy.
    async fn health_check(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

/// Trait for implementing Source Connectors (External System → Danube)
///
/// Source connectors read data from external systems and publish to Danube topics.
///
/// # Example
///
/// ```rust,no_run
/// use danube_connect_core::{SourceConnector, SourceEnvelope, ConnectorConfig, ConnectorResult, Offset, ProducerConfig};
/// use async_trait::async_trait;
/// use std::env;
///
/// pub struct FileSource {
///     file_path: String,
///     position: u64,
/// }
///
/// #[async_trait]
/// impl SourceConnector for FileSource {
///     async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
///         // Connectors now manage their own config
///         self.file_path = env::var("FILE_PATH")
///             .map_err(|_| danube_connect_core::ConnectorError::config("FILE_PATH required"))?;
///         Ok(())
///     }
///     
///     async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>> {
///         // Define destination topics and their configurations
///         Ok(vec![
///             ProducerConfig::new("/default/file_data", 0, false)
///         ])
///     }
///     
///     async fn poll(&mut self) -> ConnectorResult<Vec<SourceEnvelope>> {
///         // Read new lines from file at self.file_path
///         Ok(vec![])
///     }
///     
///     async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()> {
///         // Save file position
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait SourceConnector: Send + Sync {
    /// Initialize the connector with configuration
    ///
    /// This method is called once at startup. Use it to:
    /// - Load configuration
    /// - Establish connections
    /// - Load last checkpoint/offset
    /// - Initialize internal state
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;

    /// Get producer configurations for all topics this connector will publish to
    ///
    /// This method should return the complete list of Danube topics and their
    /// configurations (partitions, reliable dispatch) that this connector will use.
    /// The runtime will create all producers upfront based on this configuration.
    ///
    /// The connector doesn't need to know about producers - it just returns
    /// SourceRecords with topic information, and the runtime maps them to the
    /// appropriate pre-created producer.
    ///
    /// # Returns
    ///
    /// Vector of `ProducerConfig` objects, one for each destination topic.
    async fn producer_configs(&self) -> ConnectorResult<Vec<ProducerConfig>>;

    /// Select the runtime mode used by this source connector.
    ///
    /// Return `SourceConnectorMode::Polling` for connectors that periodically
    /// fetch data via `poll()`. Return `SourceConnectorMode::Streaming` for
    /// connectors that push records asynchronously via `start_streaming()`.
    fn mode(&self) -> SourceConnectorMode {
        SourceConnectorMode::Polling
    }

    /// Start a streaming source connector.
    ///
    /// Connectors that return `SourceConnectorMode::Streaming` should override
    /// this method and use the provided `SourceSender` to emit records as they
    /// become available.
    async fn start_streaming(&mut self, _sender: SourceSender) -> ConnectorResult<()> {
        Err(ConnectorError::config(
            "start_streaming() not implemented for this source connector",
        ))
    }

    /// Poll for new data from the external system
    ///
    /// This method is called repeatedly in a loop. Return:
    /// - Non-empty vector of envelopes when data is available
    /// - Empty vector when no data is available (non-blocking)
    ///
    /// The runtime will handle publishing records to Danube and calling `commit()`
    /// after successful acknowledgment.
    ///
    /// # Errors
    ///
    /// Return `ConnectorError::Retryable` for transient failures
    /// Return `ConnectorError::Fatal` to stop the connector
    async fn poll(&mut self) -> ConnectorResult<Vec<SourceEnvelope>> {
        Err(ConnectorError::config(
            "poll() not implemented for this source connector",
        ))
    }

    /// Optional: Commit offset/checkpoint after successful publish
    ///
    /// This method is called by the runtime after messages are successfully
    /// published and acknowledged by Danube. Use it to save checkpoints or
    /// acknowledge messages in the source system.
    ///
    /// # Arguments
    ///
    /// * `offsets` - List of offsets that were successfully published
    async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()> {
        let _ = offsets; // Suppress unused warning
        Ok(())
    }

    /// Optional: Called before shutdown
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        Ok(())
    }

    /// Optional: Health check implementation
    async fn health_check(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

/// Execution model for a source connector.
///
/// This enum determines how the source connector interacts with the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceConnectorMode {
    /// The runtime repeatedly calls `poll()` to fetch new data.
    Polling,
    /// The connector pushes new data through `start_streaming()`.
    Streaming,
}

/// Handle used by streaming connectors to emit records into the runtime.
///
/// This struct provides methods to send records to the runtime for publishing.
#[derive(Clone)]
pub struct SourceSender {
    sender: mpsc::Sender<SourceEnvelope>,
}

impl SourceSender {
    /// Create a new `SourceSender` instance.
    pub(crate) fn new(sender: mpsc::Sender<SourceEnvelope>) -> Self {
        Self { sender }
    }

    /// Send a single record envelope to the runtime.
    ///
    /// This method is used by streaming connectors to emit records as they become
    /// available.
    pub async fn send(&self, envelope: impl Into<SourceEnvelope>) -> ConnectorResult<()> {
        self.sender
            .send(envelope.into())
            .await
            .map_err(|e| ConnectorError::fatal(format!("failed to emit source envelope: {}", e)))
    }

    /// Send multiple record envelopes to the runtime in sequence.
    ///
    /// This method is used by streaming connectors to emit multiple records at
    /// once.
    pub async fn send_batch<I>(&self, envelopes: I) -> ConnectorResult<()>
    where
        I: IntoIterator,
        I::Item: Into<SourceEnvelope>,
    {
        for envelope in envelopes {
            self.send(envelope).await?;
        }

        Ok(())
    }

    /// Return `true` when the receiving side of the runtime channel is closed.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// A source record together with optional checkpoint information.
///
/// This struct represents a single record emitted by a source connector, along
/// with optional checkpoint information.
#[derive(Debug, Clone)]
pub struct SourceEnvelope {
    /// The record to publish to Danube.
    pub record: SourceRecord,
    /// Optional offset/checkpoint to commit after successful publish.
    pub offset: Option<Offset>,
}

impl SourceEnvelope {
    /// Create an envelope without checkpoint information.
    pub fn new(record: SourceRecord) -> Self {
        Self {
            record,
            offset: None,
        }
    }

    /// Create an envelope with an offset to commit after successful delivery.
    pub fn with_offset(record: SourceRecord, offset: Offset) -> Self {
        Self {
            record,
            offset: Some(offset),
        }
    }

    /// Borrow the record carried by this envelope.
    pub fn record(&self) -> &SourceRecord {
        &self.record
    }

    /// Borrow the offset carried by this envelope, if present.
    pub fn offset(&self) -> Option<&Offset> {
        self.offset.as_ref()
    }

    /// Split this envelope into its record and optional offset parts.
    pub fn into_parts(self) -> (SourceRecord, Option<Offset>) {
        (self.record, self.offset)
    }
}

impl From<SourceRecord> for SourceEnvelope {
    fn from(record: SourceRecord) -> Self {
        Self::new(record)
    }
}

/// Checkpoint/offset information for source connectors
///
/// **Mandatory public API** - used by source connectors for checkpointing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offset {
    /// The partition or source identifier
    pub partition: String,
    /// The offset value (interpretation depends on source)
    pub value: u64,
    /// Optional metadata
    pub metadata: Option<String>,
}

impl Offset {
    /// Create a new offset
    pub fn new(partition: impl Into<String>, value: u64) -> Self {
        Self {
            partition: partition.into(),
            value,
            metadata: None,
        }
    }

    /// Create an offset with metadata
    pub fn with_metadata(
        partition: impl Into<String>,
        value: u64,
        metadata: impl Into<String>,
    ) -> Self {
        Self {
            partition: partition.into(),
            value,
            metadata: Some(metadata.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_creation() {
        let offset = Offset::new("partition-0", 42);
        assert_eq!(offset.partition, "partition-0");
        assert_eq!(offset.value, 42);
        assert!(offset.metadata.is_none());

        let offset_with_meta = Offset::with_metadata("partition-1", 100, "some-metadata");
        assert_eq!(offset_with_meta.metadata, Some("some-metadata".to_string()));
    }
}
