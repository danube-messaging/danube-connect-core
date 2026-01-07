//! Serialization helpers for common formats.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Serialization errors
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Result type for serialization operations
pub type Result<T> = std::result::Result<T, SerializationError>;

/// JSON serialization helpers
pub mod json {
    use super::*;

    /// Serialize a value to JSON bytes
    pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(value)?)
    }

    /// Serialize a value to pretty JSON bytes
    pub fn to_bytes_pretty<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(value)?)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
        Ok(serde_json::from_slice(bytes)?)
    }

    /// Serialize to JSON string
    pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
        Ok(serde_json::to_string(value)?)
    }

    /// Deserialize from JSON string
    pub fn from_string<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T> {
        Ok(serde_json::from_str(s)?)
    }
}

/// String conversion helpers
pub mod string {
    use super::*;

    /// Convert bytes to UTF-8 string
    pub fn from_bytes(bytes: &[u8]) -> Result<&str> {
        Ok(std::str::from_utf8(bytes)?)
    }

    /// Convert bytes to owned String
    pub fn from_bytes_owned(bytes: &[u8]) -> Result<String> {
        Ok(from_bytes(bytes)?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_json_serialization() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let bytes = json::to_bytes(&data).unwrap();
        let deserialized: TestData = json::from_bytes(&bytes).unwrap();

        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_string_conversion() {
        let text = "hello world";
        let bytes = text.as_bytes();

        let converted = string::from_bytes(bytes).unwrap();
        assert_eq!(text, converted);

        let owned = string::from_bytes_owned(bytes).unwrap();
        assert_eq!(text, owned);
    }
}
