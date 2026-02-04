//! Reply source: inbound responses from communication providers

use crate::models::ResponseType;
use async_trait::async_trait;

/// Inbound reply from a provider (e.g. Telegram), to be matched to a pending prompt.
#[derive(Debug, Clone)]
pub struct ProviderReply {
    /// Provider message id this reply refers to (reply-to); None = use oldest pending
    pub reply_to_message_id: Option<String>,
    pub answer: Option<String>,
    pub response_type: ResponseType,
}

/// Source of operator replies from a provider (e.g. Telegram getUpdates).
#[async_trait]
pub trait ReplySource: Send + Sync {
    /// Poll for the next reply, if any. Returns None when no reply available.
    async fn next_reply(&self) -> Option<ProviderReply>;
}
