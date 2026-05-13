//! Client helpers for working with an Ailoop server (message and task APIs).

use crate::models::{
    DecisionOption, DecisionRecommendation, Message, MessageContent, NotificationPriority,
    SenderType,
};
use anyhow::Result;

pub mod pending_client;
pub mod task_client;

/// Send a structured decision and wait for the human's selection (returns the Response message).
#[allow(clippy::too_many_arguments)]
pub async fn ask_decision(
    server_url: &str,
    channel: &str,
    decision_id: String,
    summary: String,
    context_markdown: Option<String>,
    options: Vec<DecisionOption>,
    recommendation: Option<DecisionRecommendation>,
    timeout_secs: u32,
) -> Result<Option<Message>> {
    crate::models::validate_decision(&options, &recommendation).map_err(|e| anyhow::anyhow!(e))?;
    let message = Message::new(
        channel.to_string(),
        SenderType::Agent,
        MessageContent::Decision {
            decision_id,
            summary,
            context_markdown,
            options,
            recommendation,
            timeout_seconds: timeout_secs,
        },
    );
    crate::transport::websocket::send_message_and_wait_response(
        server_url.to_string(),
        channel.to_string(),
        message,
        timeout_secs,
    )
    .await
}

/// Request authorization through the WebSocket API and wait for a response.
pub async fn authorize(
    server_url: &str,
    channel: &str,
    action: &str,
    timeout_secs: u32,
) -> Result<Option<Message>> {
    let message = Message::new(
        channel.to_string(),
        SenderType::Agent,
        MessageContent::Authorization {
            action: action.to_string(),
            context: None,
            timeout_seconds: timeout_secs,
        },
    );

    crate::transport::websocket::send_message_and_wait_response(
        server_url.to_string(),
        channel.to_string(),
        message,
        timeout_secs,
    )
    .await
}

/// Send a notification message through the WebSocket API without waiting for a response.
pub async fn say(server_url: &str, channel: &str, text: &str, priority: &str) -> Result<()> {
    let message = Message::new(
        channel.to_string(),
        SenderType::Agent,
        MessageContent::Notification {
            text: text.to_string(),
            priority: map_priority(priority),
        },
    );

    crate::transport::websocket::send_message_no_response(
        server_url.to_string(),
        channel.to_string(),
        message,
    )
    .await
}

/// Request navigation through the WebSocket API without waiting for a response.
pub async fn navigate(server_url: &str, channel: &str, url: &str) -> Result<()> {
    let message = Message::new(
        channel.to_string(),
        SenderType::Agent,
        MessageContent::Navigate {
            url: url.to_string(),
        },
    );

    crate::transport::websocket::send_message_no_response(
        server_url.to_string(),
        channel.to_string(),
        message,
    )
    .await
}

fn map_priority(priority: &str) -> NotificationPriority {
    match priority.to_lowercase().as_str() {
        "low" => NotificationPriority::Low,
        "high" => NotificationPriority::High,
        "urgent" => NotificationPriority::Urgent,
        _ => NotificationPriority::Normal,
    }
}
