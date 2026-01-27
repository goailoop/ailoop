//! Output capture and streaming types

use serde::{Deserialize, Serialize};

/// Type of output chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkType {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
}

/// A chunk of output data from a workflow execution
///
/// Output is captured in chunks to enable streaming and efficient persistence.
/// Each chunk is sequenced and timestamped for ordering and reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputChunk {
    /// Execution ID this chunk belongs to
    pub execution_id: String,

    /// State name that produced this output
    pub state_name: String,

    /// Type of output (stdout or stderr)
    pub chunk_type: ChunkType,

    /// Sequence number for ordering chunks
    pub sequence: u64,

    /// Raw output data (bytes)
    pub data: Vec<u8>,

    /// Timestamp in milliseconds since Unix epoch
    pub timestamp: u64,
}

impl OutputChunk {
    /// Create a new stdout chunk
    pub fn new_stdout(
        execution_id: String,
        state_name: String,
        sequence: u64,
        data: Vec<u8>,
    ) -> Self {
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

    /// Create a new stderr chunk
    pub fn new_stderr(
        execution_id: String,
        state_name: String,
        sequence: u64,
        data: Vec<u8>,
    ) -> Self {
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

    /// Get the size of this chunk in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Convert chunk data to string (lossy)
    pub fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_chunk_stdout() {
        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            0,
            b"Hello, world!\n".to_vec(),
        );

        assert_eq!(chunk.execution_id, "exec-1");
        assert_eq!(chunk.state_name, "state-1");
        assert_eq!(chunk.chunk_type, ChunkType::Stdout);
        assert_eq!(chunk.sequence, 0);
        assert_eq!(chunk.size(), 14);
        assert_eq!(chunk.as_string(), "Hello, world!\n");
    }

    #[test]
    fn test_output_chunk_stderr() {
        let chunk = OutputChunk::new_stderr(
            "exec-1".to_string(),
            "state-1".to_string(),
            1,
            b"Error occurred\n".to_vec(),
        );

        assert_eq!(chunk.chunk_type, ChunkType::Stderr);
        assert_eq!(chunk.sequence, 1);
    }

    #[test]
    fn test_output_chunk_serialization() {
        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            0,
            b"test data".to_vec(),
        );

        // Serialize to JSON
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("exec-1"));
        assert!(json.contains("Stdout"));

        // Deserialize back
        let deserialized: OutputChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.execution_id, chunk.execution_id);
        assert_eq!(deserialized.chunk_type, chunk.chunk_type);
        assert_eq!(deserialized.data, chunk.data);
    }

    #[test]
    fn test_output_chunk_binary_data() {
        // Test with non-UTF8 binary data
        let binary_data = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01];
        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            0,
            binary_data.clone(),
        );

        assert_eq!(chunk.data, binary_data);
        assert_eq!(chunk.size(), 5);
    }

    #[test]
    fn test_output_chunk_timestamp() {
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let chunk = OutputChunk::new_stdout(
            "exec-1".to_string(),
            "state-1".to_string(),
            0,
            b"test".to_vec(),
        );

        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        assert!(chunk.timestamp >= before);
        assert!(chunk.timestamp <= after);
    }
}
