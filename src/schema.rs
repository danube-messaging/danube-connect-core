//! Schema handling for Danube messages.
//!
//! This module provides schema configuration, registration, and type conversion utilities.
//! All schema operations are centralized here for maintainability.

use crate::config::{SchemaConfig, SchemaMapping};
use crate::runtime::ConnectorContext;
use crate::{ConnectorError, ConnectorResult};
use danube_client::SchemaRegistryClient;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// Re-export danube_client::SchemaType as the canonical type
pub use danube_client::SchemaType;

/// Schema registry handler for connector schema operations
pub(crate) struct SchemaRegistry {
    context: Arc<ConnectorContext>,
}

impl SchemaRegistry {
    /// Create a new schema registry handler
    pub(crate) fn new(context: Arc<ConnectorContext>) -> Self {
        Self { context }
    }

    /// Initialize and register schemas from configuration
    ///
    /// Returns a map of topic -> SchemaConfig for runtime use
    pub(crate) async fn initialize(
        &self,
        schema_mappings: &[SchemaMapping],
    ) -> ConnectorResult<HashMap<String, SchemaConfig>> {
        if schema_mappings.is_empty() {
            info!("No schemas configured - messages will be sent without schema validation");
            return Ok(HashMap::new());
        }

        info!("Initializing {} schema(s)", schema_mappings.len());

        let mut schema_client = self.context.schema_client().await?;
        let mut schema_configs = HashMap::new();

        for schema_mapping in schema_mappings {
            let config = self
                .process_schema_mapping(schema_mapping, &mut schema_client)
                .await?;
            schema_configs.insert(schema_mapping.topic.clone(), config);
        }

        info!("Schema initialization complete");
        Ok(schema_configs)
    }

    /// Process a single schema mapping: load, register, and create config
    async fn process_schema_mapping(
        &self,
        mapping: &SchemaMapping,
        client: &mut SchemaRegistryClient,
    ) -> ConnectorResult<SchemaConfig> {
        let schema_type_str = mapping.schema_type.to_lowercase();

        // Load schema file if needed
        let schema_data = self.load_schema_file(mapping, &schema_type_str)?;

        // Register or verify schema
        if mapping.auto_register {
            self.register_or_verify_schema(mapping, &schema_type_str, &schema_data, client)
                .await?;
        } else {
            self.verify_schema_exists(mapping, client).await?;
        }

        info!(
            "✅ Topic '{}' configured with schema '{}' (type: {})",
            mapping.topic, mapping.subject, mapping.schema_type
        );

        Ok(SchemaConfig {
            subject: mapping.subject.clone(),
            schema_type: mapping.schema_type.clone(),
            schema_file: mapping.schema_file.clone(),
            auto_register: mapping.auto_register,
            version_strategy: mapping.version_strategy.clone(),
        })
    }

    /// Load schema file from disk if needed
    fn load_schema_file(
        &self,
        mapping: &SchemaMapping,
        schema_type_str: &str,
    ) -> ConnectorResult<Vec<u8>> {
        if requires_schema_file(schema_type_str) {
            std::fs::read(&mapping.schema_file).map_err(|e| {
                ConnectorError::config(format!(
                    "Failed to read schema file '{:?}': {}",
                    mapping.schema_file, e
                ))
            })
        } else {
            Ok(Vec::new())
        }
    }

    /// Register schema if it doesn't exist, or verify existing one
    async fn register_or_verify_schema(
        &self,
        mapping: &SchemaMapping,
        schema_type_str: &str,
        schema_data: &[u8],
        client: &mut SchemaRegistryClient,
    ) -> ConnectorResult<()> {
        match client.get_latest_schema(&mapping.subject).await {
            Ok(existing) => {
                info!(
                    "Schema '{}' already exists (ID: {}, version: {})",
                    mapping.subject, existing.schema_id, existing.version
                );
            }
            Err(_) => {
                info!("Registering new schema '{}'...", mapping.subject);

                let schema_type = parse_schema_type(schema_type_str)?;
                let _schema_id = client
                    .register_schema(&mapping.subject)
                    .with_type(schema_type)
                    .with_schema_data(schema_data.to_vec())
                    .execute()
                    .await
                    .map_err(|e| {
                        ConnectorError::fatal(format!(
                            "Failed to register schema '{}': {}",
                            mapping.subject, e
                        ))
                    })?;

                info!("✅ Schema '{}' registered successfully", mapping.subject);
            }
        }
        Ok(())
    }

    /// Verify that schema exists in registry
    async fn verify_schema_exists(
        &self,
        mapping: &SchemaMapping,
        client: &mut SchemaRegistryClient,
    ) -> ConnectorResult<()> {
        client
            .get_latest_schema(&mapping.subject)
            .await
            .map_err(|e| {
                ConnectorError::config(format!(
                    "Schema '{}' not found and auto_register=false: {}",
                    mapping.subject, e
                ))
            })?;
        info!("Schema '{}' found in registry", mapping.subject);
        Ok(())
    }
}

/// Parse a schema type string to danube_client::SchemaType
///
/// Supports various formats:
/// - "json_schema", "json" -> JsonSchema
/// - "string" -> String
/// - "number" -> Number
/// - "bytes" -> Bytes
/// - "avro" -> Avro
/// - "protobuf" -> Protobuf
fn parse_schema_type(s: &str) -> ConnectorResult<SchemaType> {
    match s.to_lowercase().as_str() {
        "json_schema" | "json" => Ok(SchemaType::JsonSchema),
        "string" => Ok(SchemaType::String),
        "number" => Ok(SchemaType::Number),
        "bytes" => Ok(SchemaType::Bytes),
        "avro" => Ok(SchemaType::Avro),
        "protobuf" => Ok(SchemaType::Protobuf),
        _ => Err(ConnectorError::config(format!(
            "Unsupported schema type: '{}'. Supported types: json_schema, json, string, number, bytes, avro, protobuf",
            s
        ))),
    }
}

/// Check if a schema type requires a schema file
///
/// Primitive types (String, Number, Bytes) don't need schema files.
/// Complex types (JsonSchema, Avro, Protobuf) require schema definitions.
fn requires_schema_file(schema_type: &str) -> bool {
    matches!(
        schema_type.to_lowercase().as_str(),
        "json_schema" | "json" | "avro" | "protobuf"
    )
}
