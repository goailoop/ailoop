//! Message converter for transforming agent events to ailoop messages

use crate::models::{Message, MessageContent, NotificationPriority, SenderType};
use crate::parser::{AgentEvent, EventType};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

/// Converts agent events to ailoop messages
///
/// This converter is completely transport-independent and agent-agnostic.
/// It preserves agent-specific metadata in the message.metadata field.
pub struct MessageConverter {
    channel: String,
    client_id: Option<String>,
    agent_type: String,
    session_id: Option<String>,
}

impl MessageConverter {
    /// Create a new message converter
    pub fn new(
        channel: String,
        client_id: Option<String>,
        agent_type: String,
    ) -> Self {
        Self {
            channel,
            client_id,
            agent_type,
            session_id: None,
        }
    }

    /// Set the session ID (extracted from system events)
    pub fn set_session_id(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    /// Set the agent type
    pub fn set_agent_type(&mut self, agent_type: String) {
        self.agent_type = agent_type;
    }

    /// Convert an agent event to one or more messages
    ///
    /// Returns a vector to handle cases where one event produces multiple messages.
    /// Preserves agent_type, session_id, client_id, and timestamp in message.metadata.
    pub fn convert(&mut self, event: AgentEvent) -> Vec<Message> {
        // Update session_id from system events
        if let EventType::System = event.event_type {
            if let Some(session_id) = event.metadata.get("session_id") {
                self.set_session_id(session_id.clone());
            }
            // System events don't produce messages, just update state
            return vec![];
        }

        // Build metadata object with all tracking information
        let mut metadata = json!({
            "agent_type": self.agent_type.clone(),
        });

        // Add session_id if available
        if let Some(ref session_id) = self.session_id {
            metadata["session_id"] = json!(session_id);
        }

        // Add client_id if available
        if let Some(ref client_id) = self.client_id {
            metadata["client_id"] = json!(client_id);
        }

        // Preserve original event metadata
        if !event.metadata.is_empty() {
            let event_metadata: HashMap<String, serde_json::Value> = event
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), json!(v)))
                .collect();
            metadata["event_metadata"] = json!(event_metadata);
        }

        // Use event timestamp if available, otherwise use current time
        let timestamp = event.timestamp.unwrap_or_else(Utc::now);

        // Convert based on event type
        let content = match event.event_type {
            EventType::Assistant => {
                let text = self.extract_text(&event.content, "message", "text");
                MessageContent::Notification {
                    text: format!("[{}] {}", self.agent_type, text),
                    priority: NotificationPriority::Normal,
                }
            }
            EventType::ToolCall => {
                let tool_name = self.extract_text(&event.content, "tool", "name");
                let status = self.extract_text(&event.content, "status", "state");
                metadata["tool_args"] = event.content.get("args").cloned().unwrap_or(json!(null));
                MessageContent::Notification {
                    text: format!("[{}] Tool: {} - {}", self.agent_type, tool_name, status),
                    priority: NotificationPriority::Low,
                }
            }
            EventType::Result => {
                let result_text = self.extract_text(&event.content, "result", "text");
                metadata["duration"] = event.content.get("duration").cloned().unwrap_or(json!(null));
                MessageContent::Notification {
                    text: format!("[{}] Result: {}", self.agent_type, result_text),
                    priority: NotificationPriority::High,
                }
            }
            EventType::User => {
                // Optional: include user events for context
                let text = self.extract_text(&event.content, "message", "text");
                MessageContent::Notification {
                    text: format!("[{}] User: {}", self.agent_type, text),
                    priority: NotificationPriority::Low,
                }
            }
            EventType::Error => {
                let error_text = self.extract_text(&event.content, "error", "message");
                metadata["error_details"] = event.content.clone();
                MessageContent::Notification {
                    text: format!("[{}] Error: {}", self.agent_type, error_text),
                    priority: NotificationPriority::Urgent,
                }
            }
            EventType::System => {
                // Already handled above
                return vec![];
            }
            EventType::Custom(typ) => {
                metadata["custom_type"] = json!(typ);
                let text = self.extract_text(&event.content, "message", "text");
                MessageContent::Notification {
                    text: format!("[{}] {}: {}", self.agent_type, typ, text),
                    priority: NotificationPriority::Normal,
                }
            }
        };

        let mut message = Message {
            id: uuid::Uuid::new_v4(),
            channel: self.channel.clone(),
            sender_type: SenderType::Agent,
            content,
            timestamp,
            correlation_id: None,
            metadata: Some(metadata),
        };

        vec![message]
    }

    /// Extract text from JSON content using multiple possible keys
    fn extract_text(&self, content: &serde_json::Value, primary: &str, secondary: &str) -> String {
        content
            .get(primary)
            .or_else(|| content.get(secondary))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Fallback: try to serialize the whole content
                serde_json::to_string(content).unwrap_or_else(|_| "".to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::EventType;
    use std::collections::HashMap;

    #[test]
    fn test_convert_assistant_event() {
        let mut converter = MessageConverter::new(
            "test-channel".to_string(),
            Some("client-123".to_string()),
            "cursor".to_string(),
        );

        let event = AgentEvent {
            agent_type: "cursor".to_string(),
            event_type: EventType::Assistant,
            content: json!({
                "message": "Hello, world!",
                "type": "assistant"
            }),
            metadata: HashMap::new(),
            timestamp: None,
        };

        let messages = converter.convert(event);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].channel, "test-channel");
        assert!(messages[0].metadata.is_some());
        let metadata = messages[0].metadata.as_ref().unwrap();
        assert_eq!(metadata["agent_type"], "cursor");
        assert_eq!(metadata["client_id"], "client-123");
    }
}
