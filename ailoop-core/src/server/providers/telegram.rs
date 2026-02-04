//! Telegram communication provider: send messages via Bot API and receive replies via getUpdates.

use crate::models::{Message, MessageContent, ResponseType};
use crate::server::providers::{NotificationSink, ProviderReply, ReplySource};
use async_trait::async_trait;
use reqwest::Client;
use std::error::Error;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";
const LONG_POLL_TIMEOUT_SECS: u64 = 30;

/// Telegram notification sink (sendMessage). Token and chat_id from config/env.
pub struct TelegramSink {
    token: String,
    chat_id: String,
    client: Arc<Client>,
}

impl TelegramSink {
    /// Create sink if token and chat_id are present. Never log token.
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            token,
            chat_id,
            client: Arc::new(Client::new()),
        }
    }

    fn format_message(message: &Message) -> String {
        let channel = &message.channel;
        let content = match &message.content {
            MessageContent::Question { text, .. } => format!("Question [{}]: {}", channel, text),
            MessageContent::Authorization { action, .. } => {
                format!("Authorization [{}]: {}", channel, action)
            }
            MessageContent::Notification { text, .. } => {
                format!("Notification [{}]: {}", channel, text)
            }
            MessageContent::Navigate { url } => format!("Navigation [{}]: {}", channel, url),
            MessageContent::Response {
                answer,
                response_type,
            } => {
                let rt = format!("{:?}", response_type);
                format!(
                    "Response [{}]: {}",
                    channel,
                    answer.as_deref().unwrap_or(rt.as_str())
                )
            }
            other => format!("[{}] {:?}", channel, other),
        };
        content
    }

    async fn send_message(&self, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!("{}{}/sendMessage", TELEGRAM_API_BASE, self.token);
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
        });
        let res = self.client.post(&url).json(&body).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Telegram API error {}: {}", status, err_body).into());
        }
        Ok(())
    }
}

#[async_trait]
impl NotificationSink for TelegramSink {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>> {
        let text = Self::format_message(message);
        self.send_message(&text).await
    }
}

// --- getUpdates (long poll) and ReplySource ---

#[derive(serde::Deserialize)]
struct GetUpdatesResponse {
    ok: bool,
    #[serde(default)]
    result: Vec<TelegramUpdate>,
}

#[derive(serde::Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    #[serde(default)]
    message: Option<TelegramMessage>,
}

#[derive(serde::Deserialize)]
struct TelegramMessage {
    #[allow(dead_code)]
    message_id: i64,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    reply_to_message: Option<ReplyToMessage>,
}

#[derive(serde::Deserialize)]
struct ReplyToMessage {
    message_id: i64,
}

/// Infers response type from text: y/yes/ok/empty -> Approved, n/no -> Denied, else Text.
/// Invalid or unparseable provider reply: for authorization/navigation treated as deny (FR-010);
/// for question the answer is used as-is (empty or error handled by caller).
pub(crate) fn infer_response_type(text: &str) -> ResponseType {
    let t = text.trim().to_lowercase();
    match t.as_str() {
        "y" | "yes" | "ok" | "" => ResponseType::AuthorizationApproved,
        "n" | "no" | "deny" | "denied" => ResponseType::AuthorizationDenied,
        _ => ResponseType::Text,
    }
}

/// Telegram reply source (getUpdates long poll). Returns replies for matching to pending prompts.
pub struct TelegramReplySource {
    token: String,
    client: Arc<Client>,
    /// Next offset for getUpdates (last_update_id + 1).
    next_offset: AtomicI64,
}

impl TelegramReplySource {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: Arc::new(Client::new()),
            next_offset: AtomicI64::new(0),
        }
    }

    /// Long poll getUpdates; returns first message as ProviderReply if any.
    async fn get_updates(&self) -> Result<Option<ProviderReply>, Box<dyn Error + Send + Sync>> {
        let offset = self.next_offset.load(Ordering::Relaxed);
        let url = format!(
            "{}{}/getUpdates?offset={}&timeout={}",
            TELEGRAM_API_BASE, self.token, offset, LONG_POLL_TIMEOUT_SECS
        );
        let res = self.client.get(&url).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("getUpdates {}: {}", status, body).into());
        }
        let body: GetUpdatesResponse = res.json().await?;
        if !body.ok || body.result.is_empty() {
            return Ok(None);
        }
        let mut last_id = offset;
        for upd in &body.result {
            last_id = upd.update_id;
            if let Some(ref msg) = upd.message {
                let text = msg.text.as_deref().unwrap_or("").to_string();
                let reply_to_message_id = msg
                    .reply_to_message
                    .as_ref()
                    .map(|r| r.message_id.to_string());
                let response_type = infer_response_type(&text);
                let reply = ProviderReply {
                    reply_to_message_id,
                    answer: Some(text),
                    response_type,
                };
                self.next_offset.store(last_id + 1, Ordering::Relaxed);
                return Ok(Some(reply));
            }
        }
        self.next_offset.store(last_id + 1, Ordering::Relaxed);
        Ok(None)
    }
}

#[async_trait]
impl ReplySource for TelegramReplySource {
    async fn next_reply(&self) -> Option<ProviderReply> {
        self.get_updates().await.ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_response_type() {
        assert_eq!(
            infer_response_type("y"),
            ResponseType::AuthorizationApproved
        );
        assert_eq!(
            infer_response_type("yes"),
            ResponseType::AuthorizationApproved
        );
        assert_eq!(
            infer_response_type("ok"),
            ResponseType::AuthorizationApproved
        );
        assert_eq!(infer_response_type(""), ResponseType::AuthorizationApproved);
        assert_eq!(infer_response_type("n"), ResponseType::AuthorizationDenied);
        assert_eq!(infer_response_type("no"), ResponseType::AuthorizationDenied);
        assert_eq!(
            infer_response_type("deny"),
            ResponseType::AuthorizationDenied
        );
        assert_eq!(infer_response_type("hello"), ResponseType::Text);
    }

    #[test]
    fn test_telegram_sink_name() {
        let sink = TelegramSink::new("token".into(), "chat".into());
        assert_eq!(sink.name(), "telegram");
    }
}
