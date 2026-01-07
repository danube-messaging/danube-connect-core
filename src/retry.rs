//! Retry strategies and backoff logic.

use std::time::Duration;

/// Configuration for retry behavior
///
/// Internal type - users configure retries via `RetrySettings` in `ConnectorConfig`.
#[derive(Debug, Clone)]
pub(crate) struct RetryConfig {
    /// Maximum number of retry attempts
    max_retries: u32,
    /// Base backoff duration in milliseconds
    base_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds
    max_backoff_ms: u64,
    /// Backoff multiplier for exponential backoff
    multiplier: f64,
    /// Add jitter to backoff to avoid thundering herd
    jitter: bool,
}

impl RetryConfig {
    /// Create a new retry configuration
    pub(crate) fn new(max_retries: u32, base_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self {
            max_retries,
            base_backoff_ms,
            max_backoff_ms,
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create exponential backoff configuration
    pub(crate) fn exponential(max_retries: u32) -> Self {
        Self {
            max_retries,
            base_backoff_ms: 1000,
            max_backoff_ms: 30000,
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create linear backoff configuration
    #[allow(dead_code)]
    pub(crate) fn linear(max_retries: u32, backoff_ms: u64) -> Self {
        Self {
            max_retries,
            base_backoff_ms: backoff_ms,
            max_backoff_ms: backoff_ms,
            multiplier: 1.0,
            jitter: false,
        }
    }

    /// Create fixed delay configuration
    #[allow(dead_code)]
    pub(crate) fn fixed(max_retries: u32, delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_backoff_ms: delay_ms,
            max_backoff_ms: delay_ms,
            multiplier: 1.0,
            jitter: false,
        }
    }

    /// Disable jitter
    #[allow(dead_code)]
    pub fn without_jitter(mut self) -> Self {
        self.jitter = false;
        self
    }

    /// Set custom multiplier
    #[allow(dead_code)]
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::exponential(3)
    }
}

/// Retry strategy implementation
///
/// Internal type - users configure retries via `RetrySettings` in `ConnectorConfig`.
#[derive(Debug, Clone)]
pub(crate) struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    /// Create a new retry strategy
    pub(crate) fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Create an exponential backoff strategy
    pub(crate) fn exponential_backoff(max_retries: u32) -> Self {
        Self::new(RetryConfig::exponential(max_retries))
    }

    /// Create a linear backoff strategy
    #[allow(dead_code)]
    pub(crate) fn linear_backoff(max_retries: u32, backoff_ms: u64) -> Self {
        Self::new(RetryConfig::linear(max_retries, backoff_ms))
    }

    /// Create a fixed delay strategy
    #[allow(dead_code)]
    pub(crate) fn fixed_delay(max_retries: u32, delay_ms: u64) -> Self {
        Self::new(RetryConfig::fixed(max_retries, delay_ms))
    }

    /// Calculate the backoff duration for a given attempt
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current attempt number (1-indexed)
    pub(crate) fn calculate_backoff(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        let mut backoff_ms =
            self.config.base_backoff_ms as f64 * self.config.multiplier.powi((attempt - 1) as i32);

        // Cap at max backoff
        backoff_ms = backoff_ms.min(self.config.max_backoff_ms as f64);

        // Add jitter if enabled
        if self.config.jitter {
            use rand::Rng;
            let jitter_factor = rand::rng().random_range(0.5..1.5);
            backoff_ms *= jitter_factor;
            // Ensure we don't exceed max after jitter
            backoff_ms = backoff_ms.min(self.config.max_backoff_ms as f64);
        }

        Duration::from_millis(backoff_ms as u64)
    }

    /// Get the maximum number of retries
    #[allow(dead_code)]
    pub(crate) fn max_retries(&self) -> u32 {
        self.config.max_retries
    }

    /// Check if should retry based on attempt count
    pub(crate) fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.config.max_retries
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::exponential_backoff(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_exponential() {
        let config = RetryConfig::exponential(5);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_backoff_ms, 1000);
        assert_eq!(config.max_backoff_ms, 30000);
        assert_eq!(config.multiplier, 2.0);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_linear() {
        let config = RetryConfig::linear(3, 500);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_backoff_ms, 500);
        assert_eq!(config.max_backoff_ms, 500);
        assert_eq!(config.multiplier, 1.0);
        assert!(!config.jitter);
    }

    #[test]
    fn test_retry_config_fixed() {
        let config = RetryConfig::fixed(10, 1000);
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.base_backoff_ms, 1000);
        assert_eq!(config.max_backoff_ms, 1000);
    }

    #[test]
    fn test_retry_strategy_exponential() {
        // Test exponential growth (without jitter for predictability)
        let strategy = RetryStrategy::new(RetryConfig::exponential(5).without_jitter());

        let backoff1 = strategy.calculate_backoff(1);
        let backoff2 = strategy.calculate_backoff(2);
        let backoff3 = strategy.calculate_backoff(3);

        assert_eq!(backoff1, Duration::from_millis(1000)); // 1000 * 2^0
        assert_eq!(backoff2, Duration::from_millis(2000)); // 1000 * 2^1
        assert_eq!(backoff3, Duration::from_millis(4000)); // 1000 * 2^2
    }

    #[test]
    fn test_retry_strategy_linear() {
        let strategy = RetryStrategy::linear_backoff(3, 500);

        let backoff1 = strategy.calculate_backoff(1);
        let backoff2 = strategy.calculate_backoff(2);
        let backoff3 = strategy.calculate_backoff(3);

        // Linear should stay constant
        assert_eq!(backoff1, Duration::from_millis(500));
        assert_eq!(backoff2, Duration::from_millis(500));
        assert_eq!(backoff3, Duration::from_millis(500));
    }

    #[test]
    fn test_retry_strategy_max_backoff() {
        let strategy = RetryStrategy::new(RetryConfig {
            max_retries: 10,
            base_backoff_ms: 1000,
            max_backoff_ms: 5000,
            multiplier: 2.0,
            jitter: false,
        });

        // Should cap at max_backoff
        let backoff10 = strategy.calculate_backoff(10);
        assert_eq!(backoff10, Duration::from_millis(5000));
    }

    #[test]
    fn test_retry_strategy_should_retry() {
        let strategy = RetryStrategy::exponential_backoff(3);

        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(1));
        assert!(strategy.should_retry(2));
        assert!(!strategy.should_retry(3));
        assert!(!strategy.should_retry(4));
    }

    #[test]
    fn test_retry_strategy_with_jitter() {
        let strategy = RetryStrategy::exponential_backoff(5);

        // With jitter, multiple calls should produce different results
        let backoff1 = strategy.calculate_backoff(2);
        let backoff2 = strategy.calculate_backoff(2);

        // They should be in a reasonable range (500ms to 3000ms for attempt 2)
        assert!(backoff1.as_millis() >= 500);
        assert!(backoff1.as_millis() <= 3000);
        assert!(backoff2.as_millis() >= 500);
        assert!(backoff2.as_millis() <= 3000);
    }
}
