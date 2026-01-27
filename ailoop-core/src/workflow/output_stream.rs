//! Output stream management for workflow execution
//!
//! Manages circular buffers for each workflow, handles output chunking,
//! persistence batching, and real-time broadcasting.

use crate::models::workflow::{ExecutionOutput, OutputType};
use crate::workflow::circular_buffer::CircularBuffer;
use crate::workflow::output::{ChunkType, OutputChunk};
use crate::workflow::persistence::WorkflowPersistence;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Maximum number of chunks to batch before persisting
const BATCH_SIZE: usize = 100;

/// Maximum bytes to buffer before persisting (1MB)
const BATCH_SIZE_BYTES: usize = 1024 * 1024;

/// Per-workflow output state
struct WorkflowOutputState {
    /// Circular buffer for recent output
    buffer: CircularBuffer<OutputChunk>,
    /// Sequence counter for chunks
    sequence: Arc<AtomicU64>,
    /// Pending chunks to persist
    pending_chunks: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,
    /// Broadcast channel for real-time streaming
    broadcast: broadcast::Sender<OutputChunk>,
}

impl WorkflowOutputState {
    fn new(buffer_capacity: usize) -> Self {
        let (broadcast, _) = broadcast::channel(1000);
        Self {
            buffer: CircularBuffer::new(buffer_capacity),
            sequence: Arc::new(AtomicU64::new(0)),
            pending_chunks: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            broadcast,
        }
    }

    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::SeqCst)
    }
}

/// Output stream manager for workflow executions
pub struct OutputStreamManager {
    /// Per-workflow output state
    workflows: Arc<DashMap<String, WorkflowOutputState>>,
    /// Persistence layer
    persistence: Arc<WorkflowPersistence>,
}

impl OutputStreamManager {
    /// Create a new output stream manager
    pub fn new(persistence: Arc<WorkflowPersistence>) -> Self {
        Self {
            workflows: Arc::new(DashMap::new()),
            persistence,
        }
    }

    /// Initialize output capture for a workflow execution
    pub fn initialize_workflow(&self, execution_id: String) {
        let state = WorkflowOutputState::new(1024 * 1024); // 1MB buffer
        self.workflows.insert(execution_id, state);
    }

    /// Subscribe to real-time output for a workflow
    pub fn subscribe(&self, execution_id: &str) -> Option<broadcast::Receiver<OutputChunk>> {
        self.workflows
            .get(execution_id)
            .map(|state| state.broadcast.subscribe())
    }

    /// Push an output chunk for a workflow
    async fn push_chunk(&self, chunk: OutputChunk) -> anyhow::Result<()> {
        if let Some(state) = self.workflows.get(&chunk.execution_id) {
            // Add to circular buffer
            let _ = state.buffer.push(chunk.clone());

            // Broadcast to subscribers
            let _ = state.broadcast.send(chunk.clone());

            // Add to pending chunks for persistence
            let mut pending = state.pending_chunks.lock().await;
            pending.push(chunk);

            // Check if we should flush batch
            let should_flush = pending.len() >= BATCH_SIZE
                || pending.iter().map(|c| c.size()).sum::<usize>() >= BATCH_SIZE_BYTES;

            if should_flush {
                let chunks_to_persist = std::mem::take(&mut *pending);
                drop(pending); // Release lock before persisting

                // Convert OutputChunk to ExecutionOutput
                let outputs: Vec<ExecutionOutput> = chunks_to_persist
                    .into_iter()
                    .map(|c| ExecutionOutput {
                        id: Uuid::new_v4(),
                        execution_id: Uuid::parse_str(&c.execution_id)
                            .unwrap_or_else(|_| Uuid::nil()),
                        state_name: c.state_name,
                        output_type: match c.chunk_type {
                            ChunkType::Stdout => OutputType::Stdout,
                            ChunkType::Stderr => OutputType::Stderr,
                        },
                        chunk_sequence: c.sequence,
                        content: c.data,
                        timestamp: Utc::now(),
                    })
                    .collect();

                // Persist batch (T074) - spawn_blocking since persistence is sync
                let persistence_clone = Arc::clone(&self.persistence);
                tokio::task::spawn_blocking(move || {
                    persistence_clone.persist_output_batch(outputs)
                })
                .await??;
            }
        }

        Ok(())
    }

    /// Stream output from an async reader (stdout or stderr)
    pub async fn stream_output<R: AsyncRead + Unpin>(
        &self,
        execution_id: String,
        state_name: String,
        chunk_type: ChunkType,
        reader: R,
    ) -> anyhow::Result<()> {
        // Get the workflow state
        let state = self
            .workflows
            .get(&execution_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not initialized"))?;

        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        // Read line by line
        loop {
            line.clear();
            let bytes_read = buf_reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                break; // EOF
            }

            // Create chunk
            let sequence = state.next_sequence();
            let chunk = match chunk_type {
                ChunkType::Stdout => OutputChunk::new_stdout(
                    execution_id.clone(),
                    state_name.clone(),
                    sequence,
                    line.as_bytes().to_vec(),
                ),
                ChunkType::Stderr => OutputChunk::new_stderr(
                    execution_id.clone(),
                    state_name.clone(),
                    sequence,
                    line.as_bytes().to_vec(),
                ),
            };

            // Push chunk
            self.push_chunk(chunk).await?;
        }

        Ok(())
    }

    /// Flush any pending chunks to persistence
    pub async fn flush_workflow(&self, execution_id: &str) -> anyhow::Result<()> {
        if let Some(state) = self.workflows.get(execution_id) {
            let mut pending = state.pending_chunks.lock().await;
            if !pending.is_empty() {
                let chunks_to_persist = std::mem::take(&mut *pending);
                drop(pending);

                // Convert OutputChunk to ExecutionOutput
                let outputs: Vec<ExecutionOutput> = chunks_to_persist
                    .into_iter()
                    .map(|c| ExecutionOutput {
                        id: Uuid::new_v4(),
                        execution_id: Uuid::parse_str(&c.execution_id)
                            .unwrap_or_else(|_| Uuid::nil()),
                        state_name: c.state_name,
                        output_type: match c.chunk_type {
                            ChunkType::Stdout => OutputType::Stdout,
                            ChunkType::Stderr => OutputType::Stderr,
                        },
                        chunk_sequence: c.sequence,
                        content: c.data,
                        timestamp: Utc::now(),
                    })
                    .collect();

                // Persist batch - spawn_blocking since persistence is sync
                let persistence_clone = Arc::clone(&self.persistence);
                tokio::task::spawn_blocking(move || {
                    persistence_clone.persist_output_batch(outputs)
                })
                .await??;
            }
        }
        Ok(())
    }

    /// Get recent output from circular buffer
    pub fn get_recent_output(&self, execution_id: &str) -> Vec<OutputChunk> {
        self.workflows
            .get(execution_id)
            .map(|state| state.buffer.iter_snapshot())
            .unwrap_or_default()
    }

    /// Cleanup workflow output state
    pub fn cleanup_workflow(&self, execution_id: &str) {
        self.workflows.remove(execution_id);
    }

    /// Get statistics for a workflow's output
    pub fn get_stats(&self, execution_id: &str) -> Option<OutputStats> {
        self.workflows.get(execution_id).map(|state| OutputStats {
            buffer_size: state.buffer.len(),
            buffer_capacity: state.buffer.capacity(),
            eviction_count: state.buffer.eviction_count(),
            sequence_number: state.sequence.load(Ordering::Relaxed),
            subscriber_count: state.broadcast.receiver_count(),
        })
    }
}

/// Statistics for workflow output capture
#[derive(Debug, Clone)]
pub struct OutputStats {
    /// Current number of chunks in buffer
    pub buffer_size: usize,
    /// Buffer capacity
    pub buffer_capacity: usize,
    /// Number of chunks evicted from buffer
    pub eviction_count: u64,
    /// Current sequence number
    pub sequence_number: u64,
    /// Number of active subscribers
    pub subscriber_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_persistence() -> (Arc<WorkflowPersistence>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("workflow_store.json");
        let persistence = Arc::new(WorkflowPersistence::new(store_path).unwrap());
        (persistence, temp_dir)
    }

    #[tokio::test]
    async fn test_output_stream_manager_initialization() {
        let (persistence, _temp_dir) = create_test_persistence();
        let manager = OutputStreamManager::new(persistence);

        manager.initialize_workflow("exec-1".to_string());

        let stats = manager.get_stats("exec-1");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().buffer_size, 0);
    }

    #[tokio::test]
    async fn test_output_stream_manager_push_chunk() {
        let (persistence, _temp_dir) = create_test_persistence();
        let manager = OutputStreamManager::new(persistence);

        manager.initialize_workflow("exec-1".to_string());

        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            0,
            b"test output\n".to_vec(),
        );

        manager.push_chunk(chunk).await.unwrap();

        let recent = manager.get_recent_output("exec-1");
        assert_eq!(recent.len(), 1);
    }

    #[tokio::test]
    async fn test_output_stream_manager_subscription() {
        let (persistence, _temp_dir) = create_test_persistence();
        let manager = OutputStreamManager::new(persistence);

        manager.initialize_workflow("exec-1".to_string());

        let mut receiver = manager.subscribe("exec-1").unwrap();

        // Push a chunk in background task
        let manager_clone = Arc::new(manager);
        let manager_ref = manager_clone.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let chunk = OutputChunk::new_stdout(
                "exec-1".to_string(),
                "state-1".to_string(),
                0,
                b"broadcast test\n".to_vec(),
            );
            manager_ref.push_chunk(chunk).await.unwrap();
        });

        // Receive the chunk
        let received = tokio::time::timeout(tokio::time::Duration::from_secs(1), receiver.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(received.execution_id, "exec-1");
        assert_eq!(received.as_string(), "broadcast test\n");
    }

    #[tokio::test]
    async fn test_output_stream_manager_flush() {
        let (persistence, _temp_dir) = create_test_persistence();
        let manager = OutputStreamManager::new(persistence);

        manager.initialize_workflow("exec-1".to_string());

        // Push a few chunks (not enough to auto-flush)
        for i in 0..10 {
            let chunk = OutputChunk::new_stdout(
                "exec-1".to_string(),
                "state-1".to_string(),
                i,
                format!("line {}\n", i).into_bytes(),
            );
            manager.push_chunk(chunk).await.unwrap();
        }

        // Manually flush
        manager.flush_workflow("exec-1").await.unwrap();

        // TODO: Verify chunks were persisted (once query_output is implemented)
    }

    #[tokio::test]
    async fn test_output_stream_manager_cleanup() {
        let (persistence, _temp_dir) = create_test_persistence();
        let manager = OutputStreamManager::new(persistence);

        manager.initialize_workflow("exec-1".to_string());
        assert!(manager.get_stats("exec-1").is_some());

        manager.cleanup_workflow("exec-1");
        assert!(manager.get_stats("exec-1").is_none());
    }
}
