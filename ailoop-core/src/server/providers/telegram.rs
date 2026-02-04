//! Telegram communication provider: send messages via Bot API and receive replies via getUpdates.

use crate::models::{Message, MessageContent, ResponseType};
use crate::server::providers::{NotificationSink, ProviderReply, ReplySource};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::error::Error;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";
const LONG_POLL_TIMEOUT_SECS: u64 = 30;
const TELEGRAM_MAX_MESSAGE_LENGTH: usize = 4096;
const SEND_RETRY_ATTEMPTS: u32 = 3;
const SEND_RETRY_BASE_DELAY_MS: u64 = 1000;
const GETUPDATES_BACKOFF_BASE_SECS: u64 = 5;
const GETUPDATES_BACKOFF_MAX_SECS: u64 = 60;
const HTTP_TIMEOUT_SECS: u64 = 30;

/// Telegram notification sink (sendMessage). Token and chat_id from config/env.
#[derive(Debug)]
pub struct TelegramSink {
    token: String,
    chat_id: String,
    client: Arc<Client>,
}

/// Response from Telegram sendMessage API
#[derive(serde::Deserialize, Debug)]
struct SendMessageResponse {
    ok: bool,
    #[serde(default)]
    result: Option<MessageResult>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
struct MessageResult {
    message_id: i64,
}

impl TelegramSink {
    /// Create sink if token and chat_id are present. Never log token.
    /// Validates chat_id format (numeric or numeric with optional "-" prefix for groups).
    pub fn new(token: String, chat_id: String) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Validate chat_id is non-empty
        if chat_id.is_empty() {
            return Err("Telegram chat_id cannot be empty".into());
        }

        // Validate chat_id format (should be numeric, optionally starting with "-" for groups)
        if !chat_id
            .chars()
            .enumerate()
            .all(|(i, c)| c.is_ascii_digit() || (i == 0 && c == '-'))
        {
            return Err(format!(
                "Telegram chat_id '{}' is not valid. Must be numeric (e.g., '123456789' or '-123456789' for groups)",
                chat_id
            )
            .into());
        }

        // Build HTTP client with timeout
        let client = Arc::new(
            Client::builder()
                .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {}", e))?,
        );

        Ok(Self {
            token,
            chat_id,
            client,
        })
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
            // Workflow and task types with human-readable formatting
            MessageContent::WorkflowProgress {
                workflow_name,
                current_state,
                status,
                progress_percentage,
                ..
            } => {
                let progress_str = progress_percentage
                    .map(|p| format!(" ({}%)", p))
                    .unwrap_or_default();
                format!(
                    "Workflow [{}]: {} – {}{} ({})",
                    channel, workflow_name, current_state, progress_str, status
                )
            }
            MessageContent::WorkflowCompleted {
                workflow_name,
                final_status,
                duration_seconds,
                ..
            } => {
                let duration_mins = duration_seconds / 60;
                let duration_secs = duration_seconds % 60;
                let duration_str = if duration_mins > 0 {
                    format!("{}m {}s", duration_mins, duration_secs)
                } else {
                    format!("{}s", duration_secs)
                };
                format!(
                    "Workflow [{}]: {} completed – {} (took {})",
                    channel, workflow_name, final_status, duration_str
                )
            }
            MessageContent::Stdout {
                execution_id,
                state_name,
                content,
                ..
            } => {
                let truncated = if content.len() > 500 {
                    format!("{}...", &content[..497])
                } else {
                    content.clone()
                };
                format!(
                    "Stdout [{}]: {}:{} – {}",
                    channel, execution_id, state_name, truncated
                )
            }
            MessageContent::Stderr {
                execution_id,
                state_name,
                content,
                ..
            } => {
                let truncated = if content.len() > 500 {
                    format!("{}...", &content[..497])
                } else {
                    content.clone()
                };
                format!(
                    "Stderr [{}]: {}:{} – {}",
                    channel, execution_id, state_name, truncated
                )
            }
            MessageContent::TaskCreate { task } => {
                format!(
                    "Task [{}]: {} created – {} (state: {})",
                    channel, task.id, task.title, task.state
                )
            }
            MessageContent::TaskUpdate { task_id, state, .. } => {
                format!("Task [{}]: {} updated – state: {}", channel, task_id, state)
            }
            MessageContent::TaskDependencyAdd {
                task_id,
                depends_on,
                dependency_type,
                ..
            } => {
                format!(
                    "Task [{}]: {} depends on {} ({:?})",
                    channel, task_id, depends_on, dependency_type
                )
            }
            MessageContent::TaskDependencyRemove {
                task_id,
                depends_on,
                ..
            } => {
                format!(
                    "Task [{}]: {} no longer depends on {}",
                    channel, task_id, depends_on
                )
            }
        };

        // Truncate if exceeds Telegram limit
        Self::truncate_message(&content)
    }

    /// Truncate message to fit Telegram's 4096 character limit
    fn truncate_message(text: &str) -> String {
        if text.len() <= TELEGRAM_MAX_MESSAGE_LENGTH {
            text.to_string()
        } else {
            format!("{}...", &text[..TELEGRAM_MAX_MESSAGE_LENGTH - 3])
        }
    }

    /// Send a message to Telegram with retry logic and return the message_id on success.
    async fn send_message_with_retry(
        &self,
        text: &str,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}{}/sendMessage", TELEGRAM_API_BASE, self.token);

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
        });

        let mut last_error: Option<Box<dyn Error + Send + Sync>> = None;

        for attempt in 0..SEND_RETRY_ATTEMPTS {
            match self.try_send_message(&url, &body).await {
                Ok(message_id) => return Ok(Some(message_id)),
                Err(e) => {
                    // Check if error is retryable
                    if Self::is_retryable_error(&*e) && attempt < SEND_RETRY_ATTEMPTS - 1 {
                        let delay_ms = SEND_RETRY_BASE_DELAY_MS * (1 << attempt); // Exponential backoff: 1s, 2s, 4s
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = SEND_RETRY_ATTEMPTS,
                            delay_ms = delay_ms,
                            error = %e,
                            "Telegram sendMessage failed, retrying..."
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "All retry attempts failed".into()))
    }

    /// Attempt to send a single message (no retry)
    async fn try_send_message(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let res = self.client.post(url).json(body).send().await?;
        let status = res.status();
        let response_text = res.text().await?;

        // Parse response
        let response: SendMessageResponse = match serde_json::from_str(&response_text) {
            Ok(r) => r,
            Err(_) => {
                return Err(format!("Telegram API error {}: {}", status, response_text).into())
            }
        };

        if !response.ok {
            let description = response.description.as_deref().unwrap_or("Unknown error");

            // Special handling for "chat not found" error
            if status == StatusCode::BAD_REQUEST
                && description.to_lowercase().contains("chat not found")
            {
                tracing::error!(
                    "Telegram chat not found: ensure the user has started the bot and chat_id is correct"
                );
                return Err(format!("Telegram chat not found ({}): verify chat_id '{}' is correct and user has started the bot", status, self.chat_id).into());
            }

            return Err(format!("Telegram API error {}: {}", status, description).into());
        }

        // Extract message_id from successful response
        match response.result {
            Some(result) => Ok(result.message_id.to_string()),
            None => Err("Telegram API returned ok=true but no result".into()),
        }
    }

    /// Determine if an error is retryable (network errors, 5xx, 429)
    fn is_retryable_error(error: &(dyn Error + Send + Sync)) -> bool {
        let error_str = error.to_string().to_lowercase();

        // Check for network-related errors
        if error_str.contains("timeout")
            || error_str.contains("connection")
            || error_str.contains("network")
            || error_str.contains("dns")
            || error_str.contains("temporarily unavailable")
        {
            return true;
        }

        // Check for HTTP 5xx or 429 in error message
        if error_str.contains(" 5")
            || error_str.contains(" 429")
            || error_str.contains("too many requests")
            || error_str.contains("rate limit")
        {
            return true;
        }

        false
    }

    /// Legacy send_message for backward compatibility (simple send without message_id)
    async fn send_message(&self, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.send_message_with_retry(text).await?;
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

    /// Send message and return Telegram message_id for reply-to matching
    async fn send_and_get_reply_to_id(
        &self,
        message: &Message,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let text = Self::format_message(message);
        self.send_message_with_retry(&text).await
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

/// Infers response type from text: y/yes/ok -> Approved, n/no -> Denied, else Text.
/// Empty string is treated as Denied for safety (changed from previous behavior).
/// Invalid or unparseable provider reply: for authorization/navigation treated as deny (FR-010);
/// for question the answer is used as-is (empty or error handled by caller).
pub(crate) fn infer_response_type(text: &str) -> ResponseType {
    let t = text.trim().to_lowercase();
    match t.as_str() {
        "y" | "yes" | "ok" => ResponseType::AuthorizationApproved,
        "n" | "no" | "deny" | "denied" | "" => ResponseType::AuthorizationDenied,
        _ => ResponseType::Text,
    }
}

/// Telegram reply source (getUpdates long poll). Returns replies for matching to pending prompts.
pub struct TelegramReplySource {
    token: String,
    client: Arc<Client>,
    /// Next offset for getUpdates (last_update_id + 1).
    next_offset: AtomicI64,
    /// Current backoff delay for error handling
    backoff_secs: AtomicI64,
}

impl TelegramReplySource {
    pub fn new(token: String) -> Self {
        // Build HTTP client with timeout
        let client = Arc::new(
            Client::builder()
                .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                .expect("Failed to build HTTP client"),
        );

        Self {
            token,
            client,
            next_offset: AtomicI64::new(0),
            backoff_secs: AtomicI64::new(GETUPDATES_BACKOFF_BASE_SECS as i64),
        }
    }

    /// Long poll getUpdates; returns first message as ProviderReply if any.
    /// Includes exponential backoff on errors.
    async fn get_updates(&self) -> Result<Option<ProviderReply>, Box<dyn Error + Send + Sync>> {
        let offset = self.next_offset.load(Ordering::Relaxed);
        let url = format!(
            "{}{}/getUpdates?offset={}&timeout={}",
            TELEGRAM_API_BASE, self.token, offset, LONG_POLL_TIMEOUT_SECS
        );

        let res = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                self.handle_error_backoff().await;
                return Err(format!("getUpdates request failed: {}", e).into());
            }
        };

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            self.handle_error_backoff().await;
            return Err(format!("getUpdates {}: {}", status, body).into());
        }

        let body: GetUpdatesResponse = match res.json().await {
            Ok(b) => b,
            Err(e) => {
                self.handle_error_backoff().await;
                return Err(format!("getUpdates JSON parse error: {}", e).into());
            }
        };

        // Success - reset backoff
        self.reset_backoff();

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

    /// Apply exponential backoff on error
    async fn handle_error_backoff(&self) {
        let current_backoff = self.backoff_secs.load(Ordering::Relaxed) as u64;
        tracing::warn!(
            "Telegram getUpdates error, backing off for {}s",
            current_backoff
        );
        sleep(Duration::from_secs(current_backoff)).await;

        // Increase backoff for next time (capped at max)
        let next_backoff = (current_backoff * 2).min(GETUPDATES_BACKOFF_MAX_SECS);
        self.backoff_secs
            .store(next_backoff as i64, Ordering::Relaxed);
    }

    /// Reset backoff after successful request
    fn reset_backoff(&self) {
        self.backoff_secs
            .store(GETUPDATES_BACKOFF_BASE_SECS as i64, Ordering::Relaxed);
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
        // Empty string now maps to Denied (safer default)
        assert_eq!(infer_response_type(""), ResponseType::AuthorizationDenied);
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
        let sink = TelegramSink::new("token".into(), "123456789".into()).unwrap();
        assert_eq!(sink.name(), "telegram");
    }

    #[test]
    fn test_chat_id_validation_empty() {
        let result = TelegramSink::new("token".into(), "".into());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot be empty"));
    }

    #[test]
    fn test_chat_id_validation_invalid() {
        let result = TelegramSink::new("token".into(), "abc123".into());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not valid"));
    }

    #[test]
    fn test_chat_id_validation_valid_numeric() {
        let result = TelegramSink::new("token".into(), "123456789".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_chat_id_validation_valid_group() {
        let result = TelegramSink::new("token".into(), "-123456789".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_truncate_message() {
        let short = "Short message";
        assert_eq!(TelegramSink::truncate_message(short), short);

        let long = "a".repeat(5000);
        let truncated = TelegramSink::truncate_message(&long);
        assert_eq!(truncated.len(), TELEGRAM_MAX_MESSAGE_LENGTH);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_format_message_workflow_progress() {
        let content = MessageContent::WorkflowProgress {
            execution_id: "exec-123".to_string(),
            workflow_name: "TestWorkflow".to_string(),
            current_state: "processing".to_string(),
            status: "running".to_string(),
            progress_percentage: Some(50),
        };
        let message = Message::new(
            "test-channel".to_string(),
            crate::models::SenderType::Agent,
            content,
        );
        let formatted = TelegramSink::format_message(&message);
        assert!(formatted.contains("Workflow"));
        assert!(formatted.contains("TestWorkflow"));
        assert!(formatted.contains("processing"));
        assert!(formatted.contains("(50%)"));
    }

    #[test]
    fn test_format_message_task_create() {
        let task = crate::models::Task::new("Test Task".to_string(), "Description".to_string());
        let content = MessageContent::TaskCreate { task };
        let message = Message::new(
            "test-channel".to_string(),
            crate::models::SenderType::Agent,
            content,
        );
        let formatted = TelegramSink::format_message(&message);
        assert!(formatted.contains("Task"));
        assert!(formatted.contains("created"));
        assert!(formatted.contains("pending"));
    }

    #[test]
    fn test_is_retryable_error() {
        let timeout_err: Box<dyn Error + Send + Sync> = "Request timeout".into();
        assert!(TelegramSink::is_retryable_error(&*timeout_err));

        let conn_err: Box<dyn Error + Send + Sync> = "Connection refused".into();
        assert!(TelegramSink::is_retryable_error(&*conn_err));

        let server_err: Box<dyn Error + Send + Sync> =
            "Telegram API error 500: Internal Server Error".into();
        assert!(TelegramSink::is_retryable_error(&*server_err));

        let rate_limit_err: Box<dyn Error + Send + Sync> =
            "Telegram API error 429: Too Many Requests".into();
        assert!(TelegramSink::is_retryable_error(&*rate_limit_err));

        let auth_err: Box<dyn Error + Send + Sync> = "Telegram API error 401: Unauthorized".into();
        assert!(!TelegramSink::is_retryable_error(&*auth_err));
    }
}
