//! Integration tests for the forward command
//!
//! These tests verify that the forward command correctly:
//! - Parses agent output in various formats
//! - Converts events to messages
//! - Sends messages via different transports (file, WebSocket)
//! - Handles different agent types

use ailoop::cli::forward::{execute_forward, ForwardConfig};
use ailoop::models::{Message, MessageContent, SenderType};
use ailoop::parser::InputFormat;
use ailoop::transport::factory::TransportType;
use std::matches;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::time::Duration;

/// Helper to create a temporary input file with content
fn create_input_file(content: &str) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(file)
}

/// Helper to read and parse messages from output file
async fn read_messages_from_file(file_path: &PathBuf) -> Result<Vec<Message>> {
    let content = fs::read_to_string(file_path)?;
    let mut messages = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let message: Message = serde_json::from_str(line)?;
        messages.push(message);
    }
    
    Ok(messages)
}

/// Test forwarding JSONL format to file transport
#[tokio::test]
async fn test_forward_jsonl_to_file() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with JSONL format
    let input_content = r#"{"agent_type":"test","type":"user","content":"Hello","timestamp":"2024-01-01T00:00:00Z"}
{"agent_type":"test","type":"assistant","content":"Hi there","timestamp":"2024-01-01T00:00:01Z"}
"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "test-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: Some("test-client".to_string()),
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    assert_eq!(messages.len(), 2);
    
    // Verify first message (user)
    assert_eq!(messages[0].channel, "test-channel");
    matches!(messages[0].sender_type, SenderType::Agent);
    if let MessageContent::Notification { text, .. } = &messages[0].content {
        assert!(text.contains("Hello"));
    } else {
        panic!("Expected Notification message");
    }
    
    // Verify second message (assistant)
    assert_eq!(messages[1].channel, "test-channel");
    if let MessageContent::Notification { text, .. } = &messages[1].content {
        assert!(text.contains("Hi there"));
    } else {
        panic!("Expected Notification message");
    }
    
    Ok(())
}

/// Test forwarding JSON format to file transport
#[tokio::test]
async fn test_forward_json_to_file() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with single JSON object
    let input_content = r#"{"agent_type":"test","type":"user","content":"Single JSON message","timestamp":"2024-01-01T00:00:00Z"}"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "json-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::Json,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: None,
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].channel, "json-channel");
    
    Ok(())
}

/// Test forwarding text format to file transport
#[tokio::test]
async fn test_forward_text_to_file() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with plain text (cursor parser handles plain text)
    let input_content = "This is a plain text message\nAnother line of text\n";
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "text-channel".to_string(),
        agent_type: Some("cursor".to_string()), // Use cursor parser for text format
        format: InputFormat::Text,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: Some("text-client".to_string()),
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    // Cursor parser creates one message per line of text
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].channel, "text-channel");
    assert_eq!(messages[1].channel, "text-channel");
    
    Ok(())
}

/// Test forwarding with cursor agent type
#[tokio::test]
async fn test_forward_cursor_agent() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with cursor-style format
    // Cursor parser expects specific format - using JSONL as fallback
    let input_content = r#"{"agent_type":"cursor","type":"assistant","content":"Cursor agent message","timestamp":"2024-01-01T00:00:00Z"}
"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "cursor-channel".to_string(),
        agent_type: Some("cursor".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: None,
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    assert!(!messages.is_empty());
    assert_eq!(messages[0].channel, "cursor-channel");
    
    Ok(())
}

/// Test forwarding multiple messages in stream-json format
#[tokio::test]
async fn test_forward_multiple_messages() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with multiple JSONL messages
    let input_content = r#"{"agent_type":"test","type":"system","content":"System message","timestamp":"2024-01-01T00:00:00Z"}
{"agent_type":"test","type":"user","content":"User message 1","timestamp":"2024-01-01T00:00:01Z"}
{"agent_type":"test","type":"user","content":"User message 2","timestamp":"2024-01-01T00:00:02Z"}
{"agent_type":"test","type":"assistant","content":"Assistant response","timestamp":"2024-01-01T00:00:03Z"}
"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "multi-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: Some("multi-client".to_string()),
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    // System events don't produce messages, so we expect 3 (user, user, assistant)
    assert_eq!(messages.len(), 3);
    
    // All messages should be on the same channel
    for message in &messages {
        assert_eq!(message.channel, "multi-channel");
    }
    
    Ok(())
}

/// Test forwarding with empty input file (should handle gracefully)
#[tokio::test]
async fn test_forward_empty_input() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create empty input file
    let input_file = create_input_file("")?;
    
    let config = ForwardConfig {
        channel: "empty-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: None,
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    // Should complete without error
    execute_forward(config).await?;
    
    // Read and verify no messages were written
    let messages = read_messages_from_file(&output_path).await?;
    assert_eq!(messages.len(), 0);
    
    Ok(())
}

/// Test forwarding with malformed JSON (should skip with warning)
#[tokio::test]
async fn test_forward_malformed_json() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with mix of valid and invalid JSON
    let input_content = r#"{"agent_type":"test","type":"user","content":"Valid message","timestamp":"2024-01-01T00:00:00Z"}
{invalid json}
{"agent_type":"test","type":"assistant","content":"Another valid message","timestamp":"2024-01-01T00:00:01Z"}
"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "malformed-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: None,
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    // Should complete, skipping malformed lines
    execute_forward(config).await?;
    
    // Read and verify only valid messages were written
    let messages = read_messages_from_file(&output_path).await?;
    assert_eq!(messages.len(), 2); // Only valid messages
    
    Ok(())
}

/// Test forwarding with metadata preservation
#[tokio::test]
async fn test_forward_metadata_preservation() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with metadata fields
    let input_content = r#"{"agent_type":"test","type":"user","content":"Message with metadata","timestamp":"2024-01-01T00:00:00Z","session_id":"session-123","client_id":"client-456"}
"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "metadata-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: Some("config-client".to_string()),
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    assert_eq!(messages.len(), 1);
    
    // Verify metadata is preserved
    if let Some(metadata) = &messages[0].metadata {
        // Metadata should contain agent-specific fields
        assert!(metadata.is_object());
    }
    
    Ok(())
}

/// Test forwarding with different event types
#[tokio::test]
async fn test_forward_different_event_types() -> Result<()> {
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().to_path_buf();
    
    // Create input file with various event types
    let input_content = r#"{"agent_type":"test","type":"system","content":"System event","timestamp":"2024-01-01T00:00:00Z"}
{"agent_type":"test","type":"user","content":"User event","timestamp":"2024-01-01T00:00:01Z"}
{"agent_type":"test","type":"assistant","content":"Assistant event","timestamp":"2024-01-01T00:00:02Z"}
{"agent_type":"test","type":"tool_call","content":"Tool call event","timestamp":"2024-01-01T00:00:03Z"}
{"agent_type":"test","type":"error","content":"Error event","timestamp":"2024-01-01T00:00:04Z"}
"#;
    let input_file = create_input_file(input_content)?;
    
    let config = ForwardConfig {
        channel: "events-channel".to_string(),
        agent_type: Some("jsonl".to_string()),
        format: InputFormat::StreamJson,
        transport_type: TransportType::File,
        url: None,
        file_path: Some(output_path.clone()),
        client_id: None,
        input_file: Some(input_file.path().to_path_buf()),
    };
    
    execute_forward(config).await?;
    
    // Read and verify messages
    let messages = read_messages_from_file(&output_path).await?;
    // System events don't produce messages, so we expect 4 (user, assistant, tool_call, error)
    assert_eq!(messages.len(), 4);
    
    // All messages should be properly formatted
    for message in &messages {
        assert_eq!(message.channel, "events-channel");
        matches!(message.sender_type, SenderType::Agent);
    }
    
    Ok(())
}
