//! Cursor CLI output parser

use crate::parser::{AgentEvent, EventType, InputFormat, AgentParser};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;

/// Parser for Cursor CLI output formats
pub struct CursorParser {
    format: InputFormat,
}

impl CursorParser {
    /// Create a new Cursor parser
    pub fn new(format: InputFormat) -> Result<Self> {
        Ok(Self { format })
    }

    /// Parse Cursor stream-json format (NDJSON)
    fn parse_stream_json(&self, line: &str) -> Result<Option<AgentEvent>> {
        let json: serde_json::Value = serde_json::from_str(line)
            .context("Failed to parse JSON line")?;

        let event_type = self.detect_event_type(&json);
        let mut metadata = HashMap::new();

        // Extract Cursor-specific metadata
        if let Some(session_id) = json.get("session_id").and_then(|v| v.as_str()) {
            metadata.insert("session_id".to_string(), session_id.to_string());
        }
        if let Some(request_id) = json.get("request_id").and_then(|v| v.as_str()) {
            metadata.insert("request_id".to_string(), request_id.to_string());
        }

        Ok(Some(AgentEvent {
            agent_type: "cursor".to_string(),
            event_type,
            content: json,
            metadata,
            timestamp: Some(Utc::now()),
        }))
    }

    /// Parse Cursor json format (single JSON object)
    fn parse_json(&self, line: &str) -> Result<Option<AgentEvent>> {
        self.parse_stream_json(line)
    }

    /// Parse Cursor text format (plain text output)
    fn parse_text(&self, line: &str) -> Result<Option<AgentEvent>> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        let content = serde_json::json!({
            "text": line,
            "format": "text"
        });

        Ok(Some(AgentEvent {
            agent_type: "cursor".to_string(),
            event_type: EventType::Assistant,
            content,
            metadata: HashMap::new(),
            timestamp: Some(Utc::now()),
        }))
    }

    /// Detect event type from Cursor JSON structure
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
            EventType::Assistant // Default for Cursor output
        }
    }
}

#[async_trait]
impl AgentParser for CursorParser {
    async fn parse_line(&mut self, line: &str) -> Result<Option<AgentEvent>> {
        match self.format {
            InputFormat::StreamJson => self.parse_stream_json(line),
            InputFormat::Json => self.parse_json(line),
            InputFormat::Text => self.parse_text(line),
        }
    }

    fn agent_type(&self) -> &str {
        "cursor"
    }

    fn supported_formats(&self) -> Vec<InputFormat> {
        vec![InputFormat::StreamJson, InputFormat::Json, InputFormat::Text]
    }
}
