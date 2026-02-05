//! OpenCode stream JSON parser

use crate::parser::{AgentEvent, AgentParser, EventType, InputFormat};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use std::collections::HashMap;

/// Parser for OpenCode stream JSON output
pub struct OpenCodeParser {
    format: InputFormat,
}

impl OpenCodeParser {
    /// Create a new OpenCode parser
    pub fn new(format: InputFormat) -> Result<Self> {
        if matches!(format, InputFormat::Text) {
            return Err(anyhow!("OpenCode does not support text format"));
        }
        Ok(Self { format })
    }

    fn parse_timestamp(value: Option<&serde_json::Value>) -> Option<DateTime<Utc>> {
        let millis = match value {
            Some(v) => {
                if let Some(num) = v.as_i64() {
                    Some(num)
                } else if let Some(text) = v.as_str() {
                    text.parse::<i64>().ok()
                } else {
                    None
                }
            }
            None => None,
        }?;

        Utc.timestamp_millis_opt(millis).single()
    }

    fn value_to_string(value: Option<&serde_json::Value>) -> String {
        match value {
            Some(v) => {
                if let Some(text) = v.as_str() {
                    text.to_string()
                } else if v.is_null() {
                    String::new()
                } else {
                    serde_json::to_string(v).unwrap_or_default()
                }
            }
            None => String::new(),
        }
    }

    fn parse_json_line(&self, line: &str) -> Result<Option<AgentEvent>> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        let json: serde_json::Value =
            serde_json::from_str(line).context("Failed to parse OpenCode JSON line")?;

        let typ = json
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing OpenCode event type"))?;

        let timestamp = Self::parse_timestamp(json.get("timestamp")).or_else(|| Some(Utc::now()));

        let mut metadata = HashMap::new();

        let event = match typ {
            "step_start" => {
                if let Some(session_id) = json.get("sessionID").and_then(|v| v.as_str()) {
                    metadata.insert("session_id".to_string(), session_id.to_string());
                }

                AgentEvent {
                    _agent_type: "opencode".to_string(),
                    event_type: EventType::System,
                    content: json!({
                        "type": "step_start"
                    }),
                    metadata,
                    timestamp,
                }
            }
            "text" => {
                let text = Self::value_to_string(
                    json.get("part")
                        .and_then(|p| p.get("text"))
                        .or_else(|| json.get("text")),
                );

                AgentEvent {
                    _agent_type: "opencode".to_string(),
                    event_type: EventType::Assistant,
                    content: json!({
                        "message": text,
                    }),
                    metadata,
                    timestamp,
                }
            }
            "tool_use" => {
                let part = json.get("part");
                let state = part.and_then(|p| p.get("state"));

                let tool_name = Self::value_to_string(part.and_then(|p| p.get("tool")));
                let status = Self::value_to_string(state.and_then(|s| s.get("status")));
                let output = Self::value_to_string(state.and_then(|s| s.get("output")));
                let title = Self::value_to_string(state.and_then(|s| s.get("title")));
                let message = if !output.is_empty() { output } else { title };
                let args = state
                    .and_then(|s| s.get("input"))
                    .cloned()
                    .unwrap_or(json!(null));

                AgentEvent {
                    _agent_type: "opencode".to_string(),
                    event_type: EventType::ToolCall,
                    content: json!({
                        "tool": tool_name,
                        "status": status,
                        "message": message,
                        "args": args,
                    }),
                    metadata,
                    timestamp,
                }
            }
            "step_finish" => {
                let part = json.get("part");
                let reason = Self::value_to_string(part.and_then(|p| p.get("reason")));

                if reason == "tool-calls" {
                    return Ok(None);
                }

                let event_type = if reason == "stop" {
                    EventType::Result
                } else {
                    EventType::Custom("step_finish".to_string())
                };

                let duration = part
                    .and_then(|p| p.get("cost"))
                    .cloned()
                    .unwrap_or(json!(null));

                let result_text = if reason == "stop" {
                    "complete".to_string()
                } else {
                    reason.clone()
                };

                AgentEvent {
                    _agent_type: "opencode".to_string(),
                    event_type,
                    content: json!({
                        "result": result_text,
                        "duration": duration,
                    }),
                    metadata,
                    timestamp,
                }
            }
            "error" => {
                let message = Self::value_to_string(
                    json.get("error")
                        .and_then(|e| e.get("data"))
                        .and_then(|d| d.get("message"))
                        .or_else(|| json.get("error").and_then(|e| e.get("message")))
                        .or_else(|| json.get("message")),
                );

                AgentEvent {
                    _agent_type: "opencode".to_string(),
                    event_type: EventType::Error,
                    content: json!({
                        "error": {
                            "message": message,
                        },
                        "message": message,
                    }),
                    metadata,
                    timestamp,
                }
            }
            other => {
                return Ok(Some(AgentEvent {
                    _agent_type: "opencode".to_string(),
                    event_type: EventType::Custom(other.to_string()),
                    content: json!({
                        "message": format!("Unsupported OpenCode event type: {}", other),
                    }),
                    metadata,
                    timestamp,
                }));
            }
        };

        Ok(Some(event))
    }
}

#[async_trait]
impl AgentParser for OpenCodeParser {
    async fn parse_line(&mut self, line: &str) -> Result<Option<AgentEvent>> {
        match self.format {
            InputFormat::StreamJson | InputFormat::Json => self.parse_json_line(line),
            InputFormat::Text => Err(anyhow!("OpenCode does not support text format")),
        }
    }

    fn agent_type(&self) -> &str {
        "opencode"
    }

    fn supported_formats(&self) -> Vec<InputFormat> {
        vec![InputFormat::StreamJson, InputFormat::Json]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[tokio::test]
    async fn test_parse_step_start() {
        let mut parser = OpenCodeParser::new(InputFormat::StreamJson).unwrap();
        let line = r#"{"type":"step_start","timestamp":1700000000000,"sessionID":"sess-1","part":{"type":"step-start","snapshot":{}}}"#;

        let event = parser.parse_line(line).await.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::System);
        assert_eq!(event._agent_type, "opencode");
        assert_eq!(event.metadata.get("session_id").unwrap(), "sess-1");
        assert_eq!(event.content.get("type").and_then(|v| v.as_str()), Some("step_start"));
        assert_eq!(
            event.timestamp,
            Some(Utc.timestamp_millis_opt(1700000000000).single().unwrap())
        );
    }

    #[tokio::test]
    async fn test_parse_text() {
        let mut parser = OpenCodeParser::new(InputFormat::StreamJson).unwrap();
        let line = r#"{"type":"text","timestamp":1700000001000,"sessionID":"sess-1","part":{"type":"text","text":"Hello"}}"#;

        let event = parser.parse_line(line).await.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::Assistant);
        assert_eq!(event.content.get("message").and_then(|v| v.as_str()), Some("Hello"));
    }

    #[tokio::test]
    async fn test_parse_tool_use() {
        let mut parser = OpenCodeParser::new(InputFormat::StreamJson).unwrap();
        let line = r#"{"type":"tool_use","timestamp":1700000002000,"sessionID":"sess-1","part":{"tool":"shell","state":{"status":"completed","input":{"cmd":"ls"},"output":"ok","title":"Shell"}}}"#;

        let event = parser.parse_line(line).await.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::ToolCall);
        assert_eq!(event.content.get("tool").and_then(|v| v.as_str()), Some("shell"));
        assert_eq!(
            event.content.get("status").and_then(|v| v.as_str()),
            Some("completed")
        );
        assert_eq!(event.content.get("message").and_then(|v| v.as_str()), Some("ok"));
        assert_eq!(
            event.content
                .get("args")
                .and_then(|v| v.get("cmd"))
                .and_then(|v| v.as_str()),
            Some("ls")
        );
    }

    #[tokio::test]
    async fn test_parse_step_finish_stop() {
        let mut parser = OpenCodeParser::new(InputFormat::StreamJson).unwrap();
        let line = r#"{"type":"step_finish","timestamp":1700000003000,"sessionID":"sess-1","part":{"type":"step-finish","reason":"stop","cost":12.3,"tokens":123}}"#;

        let event = parser.parse_line(line).await.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::Result);
        assert_eq!(event.content.get("result").and_then(|v| v.as_str()), Some("complete"));
        assert_eq!(event.content.get("duration").and_then(|v| v.as_f64()), Some(12.3));
    }

    #[tokio::test]
    async fn test_parse_error() {
        let mut parser = OpenCodeParser::new(InputFormat::StreamJson).unwrap();
        let line = r#"{"type":"error","timestamp":1700000004000,"sessionID":"sess-1","error":{"name":"ToolError","data":{"message":"boom"}}}"#;

        let event = parser.parse_line(line).await.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::Error);
        assert_eq!(event.content.get("message").and_then(|v| v.as_str()), Some("boom"));
    }
}
