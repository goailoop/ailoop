//! Notification sink: outbound delivery to communication providers

use crate::models::Message;
use async_trait::async_trait;
use std::error::Error;

/// Sink for broadcasting messages to a communication provider (e.g. Telegram).
#[async_trait]
pub trait NotificationSink: Send + Sync {
    /// Provider name for logging (e.g. "telegram").
    fn name(&self) -> &str;

    /// Send a message to the provider. Failures must not block other delivery paths.
    async fn send(&self, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send a message and return a reply-to identifier if the provider supports it.
    /// Default implementation calls `send` and returns `Ok(None)`.
    /// Used for matching replies to the correct pending prompt.
    async fn send_and_get_reply_to_id(
        &self,
        message: &Message,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        self.send(message).await?;
        Ok(None)
    }
}
