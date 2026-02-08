//! Agent event parser system
//!
//! This module provides an extensible parser system for converting agent output
//! (from Cursor CLI, Claude, GPT, etc.) into standardized AgentEvent structures
//! that can then be converted to ailoop Messages.
//!
//! ## Supported Agent Types
//!
//! - **Cursor**: Parser for Cursor CLI output formats (StreamJson, Json, Text)
//! - **Jsonl**: Generic JSONL parser that works with any agent output containing agent_type tags
//! - **OpenCode**: Parser for OpenCode stream JSON output (StreamJson, Json)
//!
//! ## Input Formats
//!
//! - `Json`: Single JSON object format
//! - `StreamJson`: Newline-delimited JSON (NDJSON) format
//! - `Text`: Plain text format (cursor only)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ailoop_core::parser::{create_parser, InputFormat};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a parser for a specific agent type
//! let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)?;
//!
//! // Parse a line of agent output
//! let line = r#"{"type":"text","timestamp":1700000000000,"part":{"text":"Hello"}}"#;
//! let event = parser.parse_line(line).await?;
//!
//! if let Some(event) = event {
//!     println!("Parsed event: {:?}", event);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Event Types
//!
//! Parsed events are classified into the following types:
//! - `System`: System-level events (step_start, etc.)
//! - `User`: User messages or input
//! - `Assistant`: Agent responses or output
//! - `ToolCall`: Tool invocation events
//! - `Result`: Operation results or completions
//! - `Error`: Error events
//! - `Custom`: Custom event types (agent-specific)

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Input format types supported by parsers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Json,
    StreamJson, // NDJSON (newline-delimited JSON)
    Text,
}

/// Event type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    System,
    User,
    Assistant,
    ToolCall,
    Result,
    Error,
    Custom(String),
}

/// Unified agent event structure
#[derive(Debug, Clone)]
pub struct AgentEvent {
    /// Agent type identifier (e.g., "cursor", "claude", "gpt")
    pub _agent_type: String,
    /// Event type classification
    pub event_type: EventType,
    /// Agent-specific event data (preserved as JSON)
    pub content: serde_json::Value,
    /// Additional metadata (tags, session_id, request_id, etc.)
    pub metadata: HashMap<String, String>,
    /// Event timestamp (if available from agent)
    pub timestamp: Option<DateTime<Utc>>,
}

/// Parser trait for agent output
#[async_trait]
pub trait AgentParser: Send + Sync {
    /// Parse a single line/event from agent output
    ///
    /// Returns `Ok(Some(event))` if a valid event was parsed,
    /// `Ok(None)` if the line should be skipped (e.g., empty or comment),
    /// or `Err` if parsing failed and should be logged.
    async fn parse_line(&mut self, line: &str) -> Result<Option<AgentEvent>>;

    /// Get agent type identifier
    fn agent_type(&self) -> &str;

    /// Get supported input formats
    #[allow(dead_code)]
    fn supported_formats(&self) -> Vec<InputFormat>;
}

/// Create a parser instance based on agent type and format
///
/// If `agent_type` is `None`, attempts to auto-detect from the format.
pub fn create_parser(
    agent_type: Option<String>,
    format: InputFormat,
) -> Result<Box<dyn AgentParser>> {
    // agent_type may be: cursor, jsonl, opencode, or None (auto-detect)
    match agent_type.as_deref() {
        Some("cursor") => crate::parser::cursor::CursorParser::new(format)
            .map(|p| Box::new(p) as Box<dyn AgentParser>),
        Some("jsonl") | None => crate::parser::jsonl::JsonlParser::new(format)
            .map(|p| Box::new(p) as Box<dyn AgentParser>),
        Some("opencode") => crate::parser::opencode::OpenCodeParser::new(format)
            .map(|p| Box::new(p) as Box<dyn AgentParser>),
        _ => anyhow::bail!("Unknown agent type: {:?}", agent_type),
    }
}

pub mod cursor;
pub mod jsonl;
pub mod opencode;
