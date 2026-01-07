//! Connector runtime context with schema registry support
//!
//! Provides centralized access to schema registry client and caching

use crate::{ConnectorError, ConnectorResult};
use danube_client::{DanubeClient, SchemaInfo, SchemaRegistryClient};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared context for connector runtime with schema registry support
///
/// Provides:
/// - Lazy-initialized schema registry client
/// - Thread-safe schema caching
/// - Automatic cache management
pub struct ConnectorContext {
    /// Danube client
    client: Arc<DanubeClient>,
    
    /// Schema registry client (lazy-initialized)
    schema_client: Arc<RwLock<Option<SchemaRegistryClient>>>,
    
    /// Schema cache: schema_id -> SchemaInfo
    /// Uses RwLock for concurrent reads with rare writes
    schema_cache: Arc<RwLock<HashMap<u64, SchemaInfo>>>,
}

impl ConnectorContext {
    /// Create a new connector context
    pub fn new(client: DanubeClient) -> Self {
        Self {
            client: Arc::new(client),
            schema_client: Arc::new(RwLock::new(None)),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get or create schema registry client (lazy initialization)
    ///
    /// The schema client is only created when first needed, avoiding
    /// overhead for connectors that don't use schema registry.
    pub async fn schema_client(&self) -> ConnectorResult<SchemaRegistryClient> {
        // Fast path: check if already initialized (read lock)
        {
            let lock = self.schema_client.read().await;
            if let Some(client) = lock.as_ref() {
                return Ok(client.clone());
            }
        }
        
        // Slow path: initialize (write lock)
        let mut lock = self.schema_client.write().await;
        
        // Double-check in case another task initialized while we waited
        if let Some(client) = lock.as_ref() {
            return Ok(client.clone());
        }
        
        // Create new schema registry client
        let client = SchemaRegistryClient::new(&self.client)
            .await
            .map_err(|e| ConnectorError::fatal(
                format!("Failed to create schema registry client: {}", e)
            ))?;
        
        *lock = Some(client.clone());
        
        tracing::info!("Schema registry client initialized");
        
        Ok(client)
    }
    
    /// Get schema by ID with caching
    ///
    /// First checks the cache, then fetches from registry if not found.
    /// Cache is automatically updated on misses.
    ///
    /// # Performance
    /// - Cache hit: ~0.1ms (just RwLock read)
    /// - Cache miss: ~5-10ms (registry lookup + cache update)
    pub async fn get_schema(&self, schema_id: u64) -> ConnectorResult<SchemaInfo> {
        // Fast path: check cache (read lock)
        {
            let cache = self.schema_cache.read().await;
            if let Some(schema) = cache.get(&schema_id) {
                tracing::trace!("Schema cache hit for ID {}", schema_id);
                return Ok(schema.clone());
            }
        }
        
        // Cache miss: fetch from registry
        tracing::debug!("Schema cache miss for ID {}, fetching from registry", schema_id);
        
        let mut client = self.schema_client().await?;
        let schema = client
            .get_schema_by_id(schema_id)
            .await
            .map_err(|e| ConnectorError::invalid_data(
                format!("Failed to fetch schema ID {}: {}", schema_id, e),
                Vec::new(),
            ))?;
        
        // Update cache (write lock)
        {
            let mut cache = self.schema_cache.write().await;
            cache.insert(schema_id, schema.clone());
            tracing::debug!(
                "Cached schema ID {} (subject: '{}', version: {})",
                schema_id,
                schema.subject,
                schema.version
            );
        }
        
        Ok(schema)
    }
    
    /// Get the Danube client
    pub fn client(&self) -> &Arc<DanubeClient> {
        &self.client
    }
    
    /// Get cache statistics (for monitoring)
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.schema_cache.read().await;
        CacheStats {
            size: cache.len(),
            capacity: cache.capacity(),
        }
    }
    
    /// Clear the schema cache (rarely needed, mainly for testing)
    pub async fn clear_cache(&self) {
        let mut cache = self.schema_cache.write().await;
        cache.clear();
        tracing::info!("Schema cache cleared");
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached schemas
    pub size: usize,
    /// Cache capacity
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: These tests would require a mock SchemaRegistryClient
    // For now, we'll test the structure compiles correctly
    
    #[test]
    fn test_cache_stats_struct() {
        let stats = CacheStats {
            size: 10,
            capacity: 100,
        };
        
        assert_eq!(stats.size, 10);
        assert_eq!(stats.capacity, 100);
    }
}
