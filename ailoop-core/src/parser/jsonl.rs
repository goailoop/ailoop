//! Generic JSONL parser for any agent output

use crate::parser::{AgentEvent, AgentParser, EventType, InputFormat};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;

/// Generic JSONL parser that can handle any agent output with agent_type tags
pub struct JsonlParser {
    format: InputFormat,
}

impl JsonlParser {
    /// Create a new JSONL parser
    pub fn new(format: InputFormat) -> Result<Self> {
        Ok(Self { format })
    }

    /// Parse a JSONL line with agent_type field
    fn parse_jsonl_line(&self, line: &str) -> Result<Option<AgentEvent>> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        let json: serde_json::Value =
            serde_json::from_str(line).context("Failed to parse JSONL line")?;

        // Extract agent_type (required for generic parser)
        let _agent_type = json
            .get("agent_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let event_type = self.detect_event_type(&json);
        let mut metadata = HashMap::new();

        // Extract common metadata fields
        if let Some(session_id) = json.get("session_id").and_then(|v| v.as_str()) {
            metadata.insert("session_id".to_string(), session_id.to_string());
        }
        if let Some(client_id) = json.get("client_id").and_then(|v| v.as_str()) {
            metadata.insert("client_id".to_string(), client_id.to_string());
        }
        if let Some(timestamp) = json.get("timestamp").and_then(|v| v.as_str()) {
            metadata.insert("timestamp".to_string(), timestamp.to_string());
        }

        // Extract timestamp if available
        let timestamp = json
            .get("timestamp")
            .and_then(|v| {
                chrono::DateTime::parse_from_rfc3339(v.as_str()?)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
            .or_else(|| Some(Utc::now()));

        Ok(Some(AgentEvent {
            _agent_type,
            event_type,
            content: json,
            metadata,
            timestamp,
        }))
    }

    /// Detect event type from JSON structure
    fn detect_event_type(&self, json: &serde_json::Value) -> EventType {
        if let Some(typ) = json.get("type").and_then(|v| v.as_str()) {
            match typ {
                "system" => EventType::System,
                "user" => EventType::User,
                "assistant" => EventType::Assistant,
                "tool_call" => EventType::ToolCall,
                "result" => EventType::Result,
                "error" => EventType::Error,
                other => EventType::Custom(other.to_string()),
            }
        } else {
            EventType::Assistant // Default
        }
    }
}

#[async_trait]
impl AgentParser for JsonlParser {
    async fn parse_line(&mut self, line: &str) -> Result<Option<AgentEvent>> {
        match self.format {
            InputFormat::StreamJson | InputFormat::Json => self.parse_jsonl_line(line),
            InputFormat::Text => {
                // For text format, try to parse as JSONL first, fallback to plain text
                if line.trim().starts_with('{') {
                    self.parse_jsonl_line(line)
                } else {
                    Ok(None) // Skip non-JSON lines in text mode
                }
            }
        }
    }

    fn agent_type(&self) -> &str {
        "jsonl"
    }

    fn supported_formats(&self) -> Vec<InputFormat> {
        vec![InputFormat::StreamJson, InputFormat::Json]
    }
}
