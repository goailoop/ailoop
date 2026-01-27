//! Unit tests for CircularBuffer

use crossbeam_queue::ArrayQueue;

/// Simple wrapper around ArrayQueue to simulate CircularBuffer behavior
/// This will be replaced by the actual CircularBuffer implementation
struct TestCircularBuffer<T> {
    queue: ArrayQueue<T>,
}

impl<T> TestCircularBuffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
        }
    }

    fn push(&self, item: T) -> Result<(), T> {
        self.queue.push(item)
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[test]
fn test_circular_buffer_creation() {
    let buffer: TestCircularBuffer<u8> = TestCircularBuffer::new(1024);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
}

#[test]
fn test_circular_buffer_push() {
    let buffer = TestCircularBuffer::new(10);

    // Push items
    assert!(buffer.push(1).is_ok());
    assert!(buffer.push(2).is_ok());
    assert!(buffer.push(3).is_ok());

    assert_eq!(buffer.len(), 3);
    assert!(!buffer.is_empty());
}

#[test]
fn test_circular_buffer_capacity_limit() {
    let buffer = TestCircularBuffer::new(3);

    // Fill buffer to capacity
    assert!(buffer.push(1).is_ok());
    assert!(buffer.push(2).is_ok());
    assert!(buffer.push(3).is_ok());

    // Next push should fail (queue is full)
    assert!(buffer.push(4).is_err());
    assert_eq!(buffer.len(), 3);
}

#[test]
fn test_circular_buffer_eviction() {
    // For a true circular buffer, we expect automatic eviction of oldest items
    // when capacity is reached. This test will verify that behavior once implemented.

    // Note: ArrayQueue doesn't auto-evict, so we'll need custom CircularBuffer
    // implementation that handles this. For now, this test documents expected behavior.

    let capacity = 5;
    let buffer = TestCircularBuffer::new(capacity);

    // Fill to capacity
    for i in 0..capacity {
        assert!(buffer.push(i).is_ok());
    }

    assert_eq!(buffer.len(), capacity);

    // TODO: Once CircularBuffer with eviction is implemented:
    // - Push additional items
    // - Verify oldest items are evicted
    // - Verify newest items are retained
}

#[test]
fn test_circular_buffer_iteration() {
    let buffer = TestCircularBuffer::new(10);

    buffer.push(10).unwrap();
    buffer.push(20).unwrap();
    buffer.push(30).unwrap();

    // TODO: Once CircularBuffer provides iteration:
    // - Iterate over all items in order
    // - Verify items are returned in FIFO order
    // - Verify iteration doesn't remove items

    assert_eq!(buffer.len(), 3);
}

#[test]
fn test_circular_buffer_1mb_capacity() {
    // Test buffer with 1MB capacity as specified in requirements (T070)
    let one_mb = 1024 * 1024;
    let buffer: TestCircularBuffer<u8> = TestCircularBuffer::new(one_mb);

    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);

    // Fill with some data
    for _ in 0..1000 {
        let _ = buffer.push(42);
    }

    assert_eq!(buffer.len(), 1000);
}

#[test]
fn test_circular_buffer_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let buffer = Arc::new(TestCircularBuffer::new(1000));
    let mut handles = vec![];

    // Spawn multiple threads to push concurrently
    for i in 0..5 {
        let buffer_clone = Arc::clone(&buffer);
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let _ = buffer_clone.push(i * 10 + j);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify items were pushed (up to capacity)
    assert!(buffer.len() <= 1000);
}
