//! Schema types for Danube messages.
//!
//! This module provides schema type definitions that mirror Danube's schema system.
//! Connectors use these to understand how to interpret message payloads.

use serde::{Deserialize, Serialize};

/// Schema type for Danube messages
///
/// Defines how message payloads are encoded and should be interpreted.
/// This mirrors the SchemaType enum from danube-client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SchemaType {
    /// Raw bytes - no specific encoding
    Bytes,
    /// UTF-8 encoded string
    String,
    /// 64-bit signed integer
    Int64,
    /// JSON-encoded data
    Json,
}

impl SchemaType {
    /// Check if this schema type represents structured data (JSON)
    pub fn is_structured(&self) -> bool {
        matches!(self, SchemaType::Json)
    }

    /// Check if this schema type represents text data
    pub fn is_text(&self) -> bool {
        matches!(self, SchemaType::String | SchemaType::Json)
    }

    /// Check if this schema type represents numeric data
    pub fn is_numeric(&self) -> bool {
        matches!(self, SchemaType::Int64)
    }

    /// Check if this schema type represents raw binary data
    pub fn is_binary(&self) -> bool {
        matches!(self, SchemaType::Bytes)
    }

    /// Get a human-readable description of this schema type
    pub fn description(&self) -> &'static str {
        match self {
            SchemaType::Bytes => "Raw binary data",
            SchemaType::String => "UTF-8 encoded string",
            SchemaType::Int64 => "64-bit signed integer",
            SchemaType::Json => "JSON-encoded structured data",
        }
    }
}

impl Default for SchemaType {
    fn default() -> Self {
        SchemaType::Bytes
    }
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaType::Bytes => write!(f, "Bytes"),
            SchemaType::String => write!(f, "String"),
            SchemaType::Int64 => write!(f, "Int64"),
            SchemaType::Json => write!(f, "Json"),
        }
    }
}

impl std::str::FromStr for SchemaType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bytes" => Ok(SchemaType::Bytes),
            "string" => Ok(SchemaType::String),
            "int64" => Ok(SchemaType::Int64),
            "json" => Ok(SchemaType::Json),
            _ => Err(format!("Unknown schema type: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_type_checks() {
        assert!(SchemaType::Json.is_structured());
        assert!(!SchemaType::String.is_structured());

        assert!(SchemaType::String.is_text());
        assert!(SchemaType::Json.is_text());
        assert!(!SchemaType::Bytes.is_text());

        assert!(SchemaType::Int64.is_numeric());
        assert!(!SchemaType::String.is_numeric());

        assert!(SchemaType::Bytes.is_binary());
        assert!(!SchemaType::Json.is_binary());
    }

    #[test]
    fn test_schema_type_from_str() {
        assert_eq!("json".parse::<SchemaType>().unwrap(), SchemaType::Json);
        assert_eq!("Json".parse::<SchemaType>().unwrap(), SchemaType::Json);
        assert_eq!("JSON".parse::<SchemaType>().unwrap(), SchemaType::Json);
        assert_eq!("bytes".parse::<SchemaType>().unwrap(), SchemaType::Bytes);
        assert_eq!("string".parse::<SchemaType>().unwrap(), SchemaType::String);
        assert_eq!("int64".parse::<SchemaType>().unwrap(), SchemaType::Int64);

        assert!("invalid".parse::<SchemaType>().is_err());
    }

    #[test]
    fn test_schema_type_display() {
        assert_eq!(SchemaType::Json.to_string(), "Json");
        assert_eq!(SchemaType::Bytes.to_string(), "Bytes");
        assert_eq!(SchemaType::String.to_string(), "String");
        assert_eq!(SchemaType::Int64.to_string(), "Int64");
    }
}
