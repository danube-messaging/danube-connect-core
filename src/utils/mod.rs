//! Utility modules for connector development.

/// Batching helpers for connector runtimes and adapters.
pub mod batching;
/// Health status helpers for implementing connector health checks.
pub mod health;
/// Lightweight serialization helpers for JSON and UTF-8 string payloads.
pub mod serialization;

// Re-export commonly used types
/// Re-export of the generic `Batcher` helper.
pub use batching::Batcher;
/// Re-exports for health monitoring helpers.
pub use health::{HealthChecker, HealthStatus};
/// Re-exports for serialization helper modules and error type.
pub use serialization::{json, string, SerializationError};
