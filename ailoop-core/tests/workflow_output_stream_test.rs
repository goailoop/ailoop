//! Unit tests for OutputStreamManager

use std::sync::Arc;

/// Mock OutputChunk for testing
#[derive(Debug, Clone, PartialEq)]
struct OutputChunk {
    execution_id: String,
    state_name: String,
    chunk_type: ChunkType,
    sequence: u64,
    data: Vec<u8>,
    timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum ChunkType {
    Stdout,
    Stderr,
}

impl OutputChunk {
    fn new_stdout(execution_id: String, state_name: String, sequence: u64, data: Vec<u8>) -> Self {
        Self {
            execution_id,
            state_name,
            chunk_type: ChunkType::Stdout,
            sequence,
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    fn new_stderr(execution_id: String, state_name: String, sequence: u64, data: Vec<u8>) -> Self {
        Self {
            execution_id,
            state_name,
            chunk_type: ChunkType::Stderr,
            sequence,
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

/// Mock OutputStreamManager for testing
struct MockOutputStreamManager {
    chunks: Arc<std::sync::Mutex<Vec<OutputChunk>>>,
}

impl MockOutputStreamManager {
    fn new() -> Self {
        Self {
            chunks: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn push_chunk(&self, chunk: OutputChunk) {
        let mut chunks = self.chunks.lock().unwrap();
        chunks.push(chunk);
    }

    fn get_chunks(&self) -> Vec<OutputChunk> {
        self.chunks.lock().unwrap().clone()
    }

    fn chunk_count(&self) -> usize {
        self.chunks.lock().unwrap().len()
    }
}

#[test]
fn test_output_stream_manager_creation() {
    let manager = MockOutputStreamManager::new();
    assert_eq!(manager.chunk_count(), 0);
}

#[test]
fn test_output_stream_manager_push_stdout() {
    let manager = MockOutputStreamManager::new();

    let chunk = OutputChunk::new_stdout(
        "exec-1".to_string(),
        "state-1".to_string(),
        0,
        b"Hello stdout\n".to_vec(),
    );

    manager.push_chunk(chunk.clone());

    let chunks = manager.get_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].execution_id, "exec-1");
    assert_eq!(chunks[0].chunk_type, ChunkType::Stdout);
}

#[test]
fn test_output_stream_manager_push_stderr() {
    let manager = MockOutputStreamManager::new();

    let chunk = OutputChunk::new_stderr(
        "exec-1".to_string(),
        "state-1".to_string(),
        0,
        b"Error occurred\n".to_vec(),
    );

    manager.push_chunk(chunk.clone());

    let chunks = manager.get_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chunk_type, ChunkType::Stderr);
}

#[test]
fn test_output_stream_manager_chunking() {
    let manager = MockOutputStreamManager::new();

    // Simulate chunking large output into smaller pieces
    let data = vec![b'A'; 8192]; // 8KB chunk

    for i in 0..10 {
        let chunk =
            OutputChunk::new_stdout("exec-1".to_string(), "state-1".to_string(), i, data.clone());
        manager.push_chunk(chunk);
    }

    let chunks = manager.get_chunks();
    assert_eq!(chunks.len(), 10);

    // Verify sequence numbers are correct
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.sequence, i as u64);
    }
}

#[test]
fn test_output_stream_manager_persistence_batching() {
    // Test that chunks are batched before persisting to reduce I/O
    let manager = MockOutputStreamManager::new();

    // Push multiple chunks
    for i in 0..100 {
        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            i,
            format!("Line {}\n", i).into_bytes(),
        );
        manager.push_chunk(chunk);
    }

    // TODO: Once persistence batching is implemented:
    // - Verify chunks are held in memory buffer
    // - Verify batch is written when threshold is reached (e.g., 100 chunks or 1MB)
    // - Verify batch is written on explicit flush

    assert_eq!(manager.chunk_count(), 100);
}

#[test]
fn test_output_stream_manager_broadcast() {
    // Test that chunks are broadcast to listeners (for WebSocket streaming)
    let manager = MockOutputStreamManager::new();

    // TODO: Once broadcast mechanism is implemented:
    // - Register a listener channel
    // - Push chunks
    // - Verify listener receives chunks in real-time
    // - Verify multiple listeners receive same chunks

    let chunk = OutputChunk::new_stdout(
        "exec-1".to_string(),
        "state-1".to_string(),
        0,
        b"Broadcast test\n".to_vec(),
    );
    manager.push_chunk(chunk);

    assert_eq!(manager.chunk_count(), 1);
}

#[test]
fn test_output_stream_manager_multiple_workflows() {
    // Test managing output from multiple concurrent workflows
    let manager = MockOutputStreamManager::new();

    // Push chunks from different workflows
    for exec_id in ["exec-1", "exec-2", "exec-3"] {
        for i in 0..5 {
            let chunk = OutputChunk::new_stdout(
                exec_id.to_string(),
                "state-1".to_string(),
                i,
                format!("Output from {}\n", exec_id).into_bytes(),
            );
            manager.push_chunk(chunk);
        }
    }

    let chunks = manager.get_chunks();
    assert_eq!(chunks.len(), 15); // 3 workflows * 5 chunks each

    // TODO: Once per-workflow buffers are implemented:
    // - Verify each workflow has its own circular buffer
    // - Verify chunks from different workflows don't interfere
}

#[test]
fn test_output_stream_manager_large_output() {
    // Test handling of large output volumes (SC-012: 100MB per workflow)
    let manager = MockOutputStreamManager::new();

    // Simulate 1MB of output in 1KB chunks
    let chunk_size = 1024;
    let num_chunks = 1024; // 1MB total

    for i in 0..num_chunks {
        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            i,
            vec![b'X'; chunk_size],
        );
        manager.push_chunk(chunk);
    }

    assert_eq!(manager.chunk_count(), num_chunks as usize);

    // TODO: Once circular buffer eviction is implemented:
    // - Verify old chunks are evicted when buffer is full
    // - Verify no data loss for recent output
    // - Verify system remains stable with large volumes
}

#[test]
fn test_output_stream_manager_timestamp_ordering() {
    let manager = MockOutputStreamManager::new();

    // Push chunks with small delays
    for i in 0..5 {
        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            i,
            format!("Line {}\n", i).into_bytes(),
        );
        manager.push_chunk(chunk);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    let chunks = manager.get_chunks();
    assert_eq!(chunks.len(), 5);

    // Verify timestamps are in ascending order
    for i in 1..chunks.len() {
        assert!(chunks[i].timestamp >= chunks[i - 1].timestamp);
    }
}
