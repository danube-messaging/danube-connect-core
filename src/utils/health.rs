//! Health check utilities for connectors.

use std::time::{Duration, Instant};

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Connector is healthy
    Healthy,
    /// Connector is degraded but operational
    Degraded,
    /// Connector is unhealthy
    Unhealthy,
}

/// Health checker with failure tracking
#[derive(Debug)]
pub struct HealthChecker {
    status: HealthStatus,
    consecutive_failures: usize,
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
    failure_threshold: usize,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(failure_threshold: usize) -> Self {
        Self {
            status: HealthStatus::Healthy,
            consecutive_failures: 0,
            last_success: Some(Instant::now()),
            last_failure: None,
            failure_threshold,
        }
    }

    /// Record a successful health check
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_success = Some(Instant::now());
        self.status = HealthStatus::Healthy;
    }

    /// Record a failed health check
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.last_failure = Some(Instant::now());

        if self.consecutive_failures >= self.failure_threshold {
            self.status = HealthStatus::Unhealthy;
        } else if self.consecutive_failures > 0 {
            self.status = HealthStatus::Degraded;
        }
    }

    /// Get the current health status
    pub fn status(&self) -> HealthStatus {
        self.status
    }

    /// Check if the connector is healthy
    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }

    /// Get consecutive failure count
    pub fn consecutive_failures(&self) -> usize {
        self.consecutive_failures
    }

    /// Get time since last success
    pub fn time_since_last_success(&self) -> Option<Duration> {
        self.last_success.map(|t| t.elapsed())
    }

    /// Get time since last failure
    pub fn time_since_last_failure(&self) -> Option<Duration> {
        self.last_failure.map(|t| t.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_checker_success() {
        let mut checker = HealthChecker::new(3);

        assert_eq!(checker.status(), HealthStatus::Healthy);
        assert!(checker.is_healthy());

        checker.record_success();
        assert_eq!(checker.status(), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_checker_degraded() {
        let mut checker = HealthChecker::new(3);

        checker.record_failure();
        assert_eq!(checker.status(), HealthStatus::Degraded);
        assert_eq!(checker.consecutive_failures(), 1);

        checker.record_failure();
        assert_eq!(checker.status(), HealthStatus::Degraded);
        assert_eq!(checker.consecutive_failures(), 2);
    }

    #[test]
    fn test_health_checker_unhealthy() {
        let mut checker = HealthChecker::new(3);

        checker.record_failure();
        checker.record_failure();
        checker.record_failure();

        assert_eq!(checker.status(), HealthStatus::Unhealthy);
        assert!(!checker.is_healthy());
        assert_eq!(checker.consecutive_failures(), 3);
    }

    #[test]
    fn test_health_checker_recovery() {
        let mut checker = HealthChecker::new(3);

        checker.record_failure();
        checker.record_failure();
        assert_eq!(checker.status(), HealthStatus::Degraded);

        checker.record_success();
        assert_eq!(checker.status(), HealthStatus::Healthy);
        assert_eq!(checker.consecutive_failures(), 0);
    }
}
