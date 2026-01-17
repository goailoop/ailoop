//! Message data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of message sender
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SenderType {
    #[serde(rename = "AGENT")]
    Agent,
    #[serde(rename = "HUMAN")]
    Human,
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "question")]
    Question {
        text: String,
        timeout_seconds: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<String>>,
    },
    #[serde(rename = "authorization")]
    Authorization {
        action: String,
        context: Option<serde_json::Value>,
        timeout_seconds: u32,
    },
    #[serde(rename = "notification")]
    Notification {
        text: String,
        #[serde(default)]
        priority: NotificationPriority,
    },
    #[serde(rename = "response")]
    Response {
        answer: Option<String>,
        response_type: ResponseType,
    },
    #[serde(rename = "navigate")]
    Navigate { url: String },
}

/// Priority levels for notifications
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum NotificationPriority {
    #[default]
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "urgent")]
    Urgent,
}

/// Types of responses to questions/authorizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "authorization_approved")]
    AuthorizationApproved,
    #[serde(rename = "authorization_denied")]
    AuthorizationDenied,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// Core message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: Uuid,
    /// Channel name (validated)
    pub channel: String,
    /// Type of sender
    pub sender_type: SenderType,
    /// Message content/payload
    pub content: MessageContent,
    /// Creation timestamp
    pub timestamp: DateTime<Utc>,
    /// Links related messages (optional)
    pub correlation_id: Option<Uuid>,
    /// Extended metadata for agent-specific and client tracking information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    /// Create a new message
    pub fn new(channel: String, sender_type: SenderType, content: MessageContent) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel,
            sender_type,
            content,
            timestamp: Utc::now(),
            correlation_id: None,
            metadata: None,
        }
    }

    /// Create a response message linked to another message
    pub fn response(channel: String, content: MessageContent, correlation_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel,
            sender_type: SenderType::Human,
            content,
            timestamp: Utc::now(),
            correlation_id: Some(correlation_id),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let content = MessageContent::Question {
            text: "What is the answer?".to_string(),
            timeout_seconds: 60,
            choices: None,
        };

        let message = Message::new("test-channel".to_string(), SenderType::Agent, content);

        assert_eq!(message.channel, "test-channel");
        assert!(matches!(message.sender_type, SenderType::Agent));
        assert!(message.correlation_id.is_none());
    }
}
