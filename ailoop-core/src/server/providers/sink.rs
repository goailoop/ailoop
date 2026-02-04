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
}
