//! Metrics and observability for connectors.

use crate::{ConnectorError, ConnectorResult};
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::OnceLock;
use std::time::Duration;

static METRICS_EXPORTER_PORT: OnceLock<u16> = OnceLock::new();
static METRICS_EXPORTER_INIT: OnceLock<Result<(), String>> = OnceLock::new();

/// Metrics collector for connectors
///
/// Provides Prometheus metrics for monitoring connector health and performance.
/// Users can create their own instances for custom metrics or use the runtime-provided instance.
#[derive(Debug, Clone)]
pub struct ConnectorMetrics {
    /// Connector name for labeling
    connector_name: String,
    /// Topic name for labeling
    topic: String,
}

impl ConnectorMetrics {
    /// Initialize the global Prometheus exporter on the given port.
    ///
    /// This method is idempotent for the same port and returns an error if the
    /// exporter was already initialized on a different port.
    pub fn initialize_exporter(port: u16) -> ConnectorResult<()> {
        let existing_port = METRICS_EXPORTER_PORT.get_or_init(|| port);
        if *existing_port != port {
            return Err(ConnectorError::config(format!(
                "metrics exporter already initialized on port {}, cannot reinitialize on port {}",
                existing_port, port
            )));
        }

        let init_result = METRICS_EXPORTER_INIT.get_or_init(|| {
            let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);

            PrometheusBuilder::new()
                .with_http_listener(listen_addr)
                .install()
                .map_err(|e| {
                    format!(
                        "failed to install Prometheus metrics exporter on {}: {}",
                        listen_addr, e
                    )
                })
        });

        match init_result {
            Ok(()) => Ok(()),
            Err(message) => Err(ConnectorError::fatal(message.clone())),
        }
    }

    /// Create a new metrics collector
    pub fn new(connector_name: impl Into<String>, topic: impl Into<String>) -> Self {
        let connector_name = connector_name.into();
        let topic = topic.into();

        // Register metric descriptions
        Self::register_metrics();

        Self {
            connector_name,
            topic,
        }
    }

    /// Register metric descriptions
    fn register_metrics() {
        // Counters
        describe_counter!(
            "danube_connector_messages_received_total",
            "Total number of messages received by the connector"
        );
        describe_counter!(
            "danube_connector_messages_processed_total",
            "Total number of messages successfully processed"
        );
        describe_counter!(
            "danube_connector_messages_failed_total",
            "Total number of messages that failed processing"
        );
        describe_counter!(
            "danube_connector_messages_retried_total",
            "Total number of message processing retries"
        );

        // Histograms
        describe_histogram!(
            "danube_connector_processing_duration_seconds",
            "Time spent processing each message"
        );
        describe_histogram!(
            "danube_connector_batch_size",
            "Number of messages in each batch"
        );

        // Gauges
        describe_gauge!(
            "danube_connector_inflight_messages",
            "Current number of messages being processed"
        );
        describe_gauge!(
            "danube_connector_health",
            "Connector health status (1 = healthy, 0 = unhealthy)"
        );
    }

    /// Record a message received
    pub fn record_received(&self) {
        counter!(
            "danube_connector_messages_received_total",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .increment(1);
    }

    /// Record a message successfully processed
    pub fn record_success(&self) {
        counter!(
            "danube_connector_messages_processed_total",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .increment(1);
    }

    /// Record a message processing failure
    pub fn record_error(&self, error_type: &str) {
        counter!(
            "danube_connector_messages_failed_total",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
            "error_type" => error_type.to_string(),
        )
        .increment(1);
    }

    /// Record a retry attempt
    pub fn record_retry(&self) {
        counter!(
            "danube_connector_messages_retried_total",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .increment(1);
    }

    /// Record processing duration
    pub fn record_processing_time(&self, duration: Duration) {
        histogram!(
            "danube_connector_processing_duration_seconds",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .record(duration.as_secs_f64());
    }

    /// Record batch size
    pub fn record_batch_size(&self, size: usize) {
        histogram!(
            "danube_connector_batch_size",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .record(size as f64);
    }

    /// Set inflight message count
    pub fn set_inflight(&self, count: usize) {
        gauge!(
            "danube_connector_inflight_messages",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .set(count as f64);
    }

    /// Increment inflight message count
    pub fn increment_inflight(&self) {
        gauge!(
            "danube_connector_inflight_messages",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .increment(1.0);
    }

    /// Decrement inflight message count
    pub fn decrement_inflight(&self) {
        gauge!(
            "danube_connector_inflight_messages",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .decrement(1.0);
    }

    /// Set connector health status
    pub fn set_health(&self, healthy: bool) {
        gauge!(
            "danube_connector_health",
            "connector" => self.connector_name.clone(),
            "topic" => self.topic.clone(),
        )
        .set(if healthy { 1.0 } else { 0.0 });
    }
}

/// Timer for tracking processing duration
///
/// This timer starts immediately when created and increments the inflight gauge.
/// When stopped, it records the elapsed duration and decrements the inflight count.
#[allow(dead_code)]
pub struct ProcessingTimer {
    start: std::time::Instant,
    metrics: ConnectorMetrics,
}

#[allow(dead_code)]
impl ProcessingTimer {
    /// Create a timer that starts immediately and increments the inflight gauge.
    pub fn new(metrics: ConnectorMetrics) -> Self {
        metrics.increment_inflight();
        Self {
            start: std::time::Instant::now(),
            metrics,
        }
    }

    /// Stop the timer, record the elapsed duration, and decrement inflight count.
    pub fn stop(self) {
        let duration = self.start.elapsed();
        self.metrics.record_processing_time(duration);
        self.metrics.decrement_inflight();
    }
}

impl Drop for ProcessingTimer {
    fn drop(&mut self) {
        // Ensure inflight is decremented even if stop() wasn't called
        self.metrics.decrement_inflight();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ConnectorMetrics::new("test-connector", "/default/test");
        assert_eq!(metrics.connector_name, "test-connector");
        assert_eq!(metrics.topic, "/default/test");
    }

    #[test]
    fn test_timer() {
        let metrics = ConnectorMetrics::new("test-connector", "/default/test");

        let timer = ProcessingTimer::new(metrics.clone());
        std::thread::sleep(Duration::from_millis(10));
        timer.stop();

        // Timer should have recorded the duration
        // We can't assert the actual value but can verify it doesn't panic
    }
}
