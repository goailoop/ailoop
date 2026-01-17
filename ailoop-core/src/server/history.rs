//! Message history storage with per-channel FIFO eviction

use crate::models::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of messages to store per channel
const MAX_MESSAGES_PER_CHANNEL: usize = 1000;

/// Message history storage with per-channel FIFO eviction
#[derive(Clone)]
pub struct MessageHistory {
    inner: Arc<RwLock<HashMap<String, VecDeque<Message>>>>,
}

impl MessageHistory {
    /// Create a new message history
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a message to the history for a channel
    pub async fn add_message(&self, channel: &str, message: Message) {
        let mut history = self.inner.write().await;
        let channel_messages = history
            .entry(channel.to_string())
            .or_insert_with(VecDeque::new);

        // Add message
        channel_messages.push_back(message);

        // Evict oldest messages if limit exceeded (FIFO)
        while channel_messages.len() > MAX_MESSAGES_PER_CHANNEL {
            channel_messages.pop_front();
        }
    }

    /// Get recent messages for a channel
    pub async fn get_messages(&self, channel: &str, limit: Option<usize>) -> Vec<Message> {
        let history = self.inner.read().await;
        if let Some(messages) = history.get(channel) {
            let limit = limit.unwrap_or(MAX_MESSAGES_PER_CHANNEL);
            messages.iter().rev().take(limit).rev().cloned().collect()
        } else {
            vec![]
        }
    }

    /// Get all channels with messages
    pub async fn get_channels(&self) -> Vec<String> {
        let history = self.inner.read().await;
        history.keys().cloned().collect()
    }

    /// Get the total number of channels
    pub async fn get_channel_count(&self) -> usize {
        let history = self.inner.read().await;
        history.len()
    }

    /// Get message count for a channel
    pub async fn get_message_count(&self, channel: &str) -> usize {
        let history = self.inner.read().await;
        history.get(channel).map(|v| v.len()).unwrap_or(0)
    }

    /// Get statistics for a channel
    pub async fn get_channel_stats(&self, channel: &str) -> ChannelStats {
        let history = self.inner.read().await;
        if let Some(messages) = history.get(channel) {
            ChannelStats {
                channel: channel.to_string(),
                message_count: messages.len(),
                oldest_message: messages.front().map(|m| m.timestamp),
                newest_message: messages.back().map(|m| m.timestamp),
            }
        } else {
            ChannelStats {
                channel: channel.to_string(),
                message_count: 0,
                oldest_message: None,
                newest_message: None,
            }
        }
    }

    /// Get a message by its ID
    pub async fn get_message_by_id(&self, message_id: &uuid::Uuid) -> Option<Message> {
        let history = self.inner.read().await;
        for messages in history.values() {
            for message in messages {
                if &message.id == message_id {
                    return Some(message.clone());
                }
            }
        }
        None
    }
}

impl Default for MessageHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a channel
#[derive(Debug, Clone)]
pub struct ChannelStats {
    pub channel: String,
    pub message_count: usize,
    pub oldest_message: Option<chrono::DateTime<chrono::Utc>>,
    pub newest_message: Option<chrono::DateTime<chrono::Utc>>,
}

use std::collections::VecDeque;
