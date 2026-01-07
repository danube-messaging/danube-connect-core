//! Error types for connector operations.

use thiserror::Error;

/// Result type for connector operations
///
/// **Mandatory public API** - all connector methods return this.
pub type ConnectorResult<T> = Result<T, ConnectorError>;

/// Error types for connector operations
///
/// **Mandatory public API** - all connectors use this for error handling.
#[derive(Error, Debug)]
pub enum ConnectorError {
    /// Retryable errors - transient failures that should be retried
    ///
    /// Examples: network timeouts, temporary service unavailability, rate limits
    #[error("Retryable error: {message}")]
    Retryable {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Fatal errors - permanent failures that require restart or configuration change
    ///
    /// Examples: authentication failures, invalid configuration, incompatible versions
    #[error("Fatal error: {message}")]
    Fatal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Invalid data - message should be skipped or sent to dead-letter queue
    ///
    /// Examples: malformed JSON, schema validation failure, corrupt data
    #[error("Invalid data: {message}")]
    InvalidData { message: String, payload: Vec<u8> },

    /// Configuration error - detected at startup
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Danube client error
    #[error("Danube error: {0}")]
    Danube(#[from] danube_client::errors::DanubeError),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl ConnectorError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, ConnectorError::Retryable { .. })
    }

    /// Check if this error is fatal
    pub fn is_fatal(&self) -> bool {
        matches!(self, ConnectorError::Fatal { .. })
    }

    /// Check if this error is due to invalid data
    pub fn is_invalid_data(&self) -> bool {
        matches!(self, ConnectorError::InvalidData { .. })
    }

    /// Create a retryable error from a message
    pub fn retryable(message: impl Into<String>) -> Self {
        ConnectorError::Retryable {
            message: message.into(),
            source: None,
        }
    }

    /// Create a retryable error with source
    pub fn retryable_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ConnectorError::Retryable {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a fatal error from a message
    pub fn fatal(message: impl Into<String>) -> Self {
        ConnectorError::Fatal {
            message: message.into(),
            source: None,
        }
    }

    /// Create a fatal error with source
    pub fn fatal_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ConnectorError::Fatal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an invalid data error
    pub fn invalid_data(message: impl Into<String>, payload: Vec<u8>) -> Self {
        ConnectorError::InvalidData {
            message: message.into(),
            payload,
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        ConnectorError::Configuration(message.into())
    }
}

// Conversion from serde_json::Error
impl From<serde_json::Error> for ConnectorError {
    fn from(err: serde_json::Error) -> Self {
        ConnectorError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let retryable = ConnectorError::retryable("network timeout");
        assert!(retryable.is_retryable());
        assert!(!retryable.is_fatal());

        let fatal = ConnectorError::fatal("auth failed");
        assert!(!fatal.is_retryable());
        assert!(fatal.is_fatal());

        let invalid = ConnectorError::invalid_data("bad json", vec![1, 2, 3]);
        assert!(invalid.is_invalid_data());
    }

    #[test]
    fn test_error_display() {
        let err = ConnectorError::retryable("test error");
        assert_eq!(err.to_string(), "Retryable error: test error");
    }
}
