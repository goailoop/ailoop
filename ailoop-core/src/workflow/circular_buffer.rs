//! Circular buffer for output capture with automatic eviction
//!
//! Uses crossbeam_queue::ArrayQueue for lock-free concurrent access.
//! When the buffer is full, oldest items are evicted to make room for new ones.

use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Circular buffer with automatic eviction of oldest items
///
/// Default capacity: 1MB (T070 requirement)
pub struct CircularBuffer<T> {
    queue: Arc<ArrayQueue<T>>,
    eviction_count: Arc<AtomicU64>,
}

impl<T> CircularBuffer<T> {
    /// Create a new circular buffer with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            eviction_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new circular buffer with 1MB capacity (default for output capture)
    pub fn new_1mb() -> Self
    where
        T: Default,
    {
        Self::new(1024 * 1024)
    }

    /// Push an item into the buffer
    ///
    /// If the buffer is full, attempts to evict the oldest item and retry.
    /// Returns Ok(()) if successful, Err(item) if eviction failed.
    pub fn push(&self, item: T) -> Result<(), T> {
        match self.queue.push(item) {
            Ok(()) => Ok(()),
            Err(item) => {
                // Buffer is full, try to evict oldest item
                if let Some(_evicted) = self.queue.pop() {
                    self.eviction_count.fetch_add(1, Ordering::Relaxed);
                    // Retry push after eviction
                    self.queue.push(item)
                } else {
                    // Eviction failed (shouldn't happen in normal operation)
                    Err(item)
                }
            }
        }
    }

    /// Try to push an item without eviction
    ///
    /// Returns Ok(()) if successful, Err(item) if buffer is full
    pub fn try_push(&self, item: T) -> Result<(), T> {
        self.queue.push(item)
    }

    /// Pop the oldest item from the buffer
    pub fn pop(&self) -> Option<T> {
        self.queue.pop()
    }

    /// Get the current number of items in the buffer
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get the total number of items that have been evicted
    pub fn eviction_count(&self) -> u64 {
        self.eviction_count.load(Ordering::Relaxed)
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    /// Check if the buffer is full
    pub fn is_full(&self) -> bool {
        self.queue.len() == self.queue.capacity()
    }

    /// Iterate over all items in the buffer without removing them
    ///
    /// Note: This creates a snapshot by draining and repopulating the buffer.
    /// During this operation, the buffer should not be concurrently modified.
    pub fn iter_snapshot(&self) -> Vec<T>
    where
        T: Clone,
    {
        let mut items = Vec::new();
        let mut temp = Vec::new();

        // Drain all items
        while let Some(item) = self.queue.pop() {
            temp.push(item);
        }

        // Clone items for return and restore to queue
        for item in &temp {
            items.push(item.clone());
        }

        // Restore items to queue
        for item in temp {
            let _ = self.queue.push(item);
        }

        items
    }

    /// Clear all items from the buffer
    pub fn clear(&self) {
        while self.queue.pop().is_some() {}
    }
}

impl<T> Clone for CircularBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
            eviction_count: Arc::clone(&self.eviction_count),
        }
    }
}

impl<T> Default for CircularBuffer<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new_1mb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_buffer_creation() {
        let buffer: CircularBuffer<u8> = CircularBuffer::new(1024);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.capacity(), 1024);
    }

    #[test]
    fn test_circular_buffer_push_and_pop() {
        let buffer = CircularBuffer::new(10);

        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert!(buffer.push(3).is_ok());

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_circular_buffer_auto_eviction() {
        let buffer = CircularBuffer::new(3);

        // Fill to capacity
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert!(buffer.push(3).is_ok());
        assert!(buffer.is_full());

        // Push one more - should evict oldest (1)
        assert!(buffer.push(4).is_ok());
        assert_eq!(buffer.eviction_count(), 1);

        // Verify oldest was evicted
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_circular_buffer_multiple_evictions() {
        let buffer = CircularBuffer::new(3);

        // Fill buffer
        for i in 0..3 {
            assert!(buffer.push(i).is_ok());
        }

        // Push 5 more items, should evict 5 oldest
        for i in 3..8 {
            assert!(buffer.push(i).is_ok());
        }

        assert_eq!(buffer.eviction_count(), 5);
        assert_eq!(buffer.len(), 3);

        // Verify only newest items remain
        assert_eq!(buffer.pop(), Some(5));
        assert_eq!(buffer.pop(), Some(6));
        assert_eq!(buffer.pop(), Some(7));
    }

    #[test]
    fn test_circular_buffer_try_push() {
        let buffer = CircularBuffer::new(2);

        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());

        // Should fail without eviction
        assert!(buffer.try_push(3).is_err());
        assert_eq!(buffer.eviction_count(), 0);
    }

    #[test]
    fn test_circular_buffer_clear() {
        let buffer = CircularBuffer::new(10);

        buffer.push(1).unwrap();
        buffer.push(2).unwrap();
        buffer.push(3).unwrap();

        assert_eq!(buffer.len(), 3);

        buffer.clear();

        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_circular_buffer_concurrent_access() {
        use std::thread;

        let buffer = CircularBuffer::new(1000);
        let mut handles = vec![];

        // Spawn multiple threads to push concurrently
        for i in 0..5 {
            let buffer_clone = buffer.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let _ = buffer_clone.push(i * 100 + j);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all items were pushed (500 items, capacity 1000)
        assert_eq!(buffer.len(), 500);
    }

    #[test]
    fn test_circular_buffer_iter_snapshot() {
        let buffer = CircularBuffer::new(10);

        buffer.push(10).unwrap();
        buffer.push(20).unwrap();
        buffer.push(30).unwrap();

        let snapshot = buffer.iter_snapshot();
        assert_eq!(snapshot, vec![10, 20, 30]);

        // Verify buffer still contains items after snapshot
        assert_eq!(buffer.len(), 3);
    }
}
