//! Batching utilities for efficient message processing.

use std::time::{Duration, Instant};

/// A generic batcher that collects items until reaching a size or timeout threshold
#[derive(Debug)]
pub struct Batcher<T> {
    items: Vec<T>,
    max_size: usize,
    timeout: Duration,
    last_flush: Instant,
}

impl<T> Batcher<T> {
    /// Create a new batcher
    pub fn new(max_size: usize, timeout: Duration) -> Self {
        Self {
            items: Vec::with_capacity(max_size),
            max_size,
            timeout,
            last_flush: Instant::now(),
        }
    }

    /// Add an item to the batch
    pub fn add(&mut self, item: T) {
        self.items.push(item);
    }

    /// Check if the batch should be flushed
    pub fn should_flush(&self) -> bool {
        self.items.len() >= self.max_size || self.last_flush.elapsed() >= self.timeout
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the current batch size
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Flush the batch and return all items
    pub fn flush(&mut self) -> Vec<T> {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.items)
    }

    /// Get a reference to the items without flushing
    pub fn items(&self) -> &[T] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_batcher_size() {
        let mut batcher = Batcher::new(3, Duration::from_secs(10));

        assert!(!batcher.should_flush());

        batcher.add(1);
        batcher.add(2);
        assert!(!batcher.should_flush());

        batcher.add(3);
        assert!(batcher.should_flush());

        let items = batcher.flush();
        assert_eq!(items, vec![1, 2, 3]);
        assert!(!batcher.should_flush());
    }

    #[test]
    fn test_batcher_timeout() {
        let mut batcher = Batcher::new(10, Duration::from_millis(50));

        batcher.add(1);
        assert!(!batcher.should_flush());

        thread::sleep(Duration::from_millis(60));
        assert!(batcher.should_flush());

        let items = batcher.flush();
        assert_eq!(items, vec![1]);
    }

    #[test]
    fn test_batcher_empty() {
        let batcher: Batcher<i32> = Batcher::new(10, Duration::from_secs(1));
        assert!(batcher.is_empty());
        assert_eq!(batcher.len(), 0);
    }
}
