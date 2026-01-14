//! Channel isolation mechanisms

use crate::models::Message;
use crate::channel::manager::ChannelManager;
use std::sync::{Arc, Mutex};

/// Thread-safe channel isolation wrapper
pub struct ChannelIsolation {
    manager: Arc<Mutex<ChannelManager>>,
}

impl ChannelIsolation {
    /// Create a new channel isolation wrapper
    pub fn new(default_channel: String) -> Self {
        Self {
            manager: Arc::new(Mutex::new(ChannelManager::new(default_channel))),
        }
    }

    /// Enqueue a message in a specific channel
    pub fn enqueue_message(&self, channel_name: &str, message: Message) {
        if let Ok(mut manager) = self.manager.lock() {
            manager.enqueue_message(channel_name, message);
        } else {
            eprintln!("Failed to acquire channel manager lock");
        }
    }

    /// Dequeue a message from a specific channel
    pub fn dequeue_message(&self, channel_name: &str) -> Option<Message> {
        if let Ok(mut manager) = self.manager.lock() {
            manager.dequeue_message(channel_name)
        } else {
            eprintln!("Failed to acquire channel manager lock");
            None
        }
    }

    /// Get queue size for a specific channel
    pub fn get_queue_size(&self, channel_name: &str) -> usize {
        if let Ok(manager) = self.manager.lock() {
            manager.get_queue_size(channel_name)
        } else {
            eprintln!("Failed to acquire channel manager lock");
            0
        }
    }

    /// Add a connection to a specific channel
    pub fn add_connection(&self, channel_name: &str) {
        if let Ok(mut manager) = self.manager.lock() {
            manager.add_connection(channel_name);
        } else {
            eprintln!("Failed to acquire channel manager lock");
        }
    }

    /// Remove a connection from a specific channel
    pub fn remove_connection(&self, channel_name: &str) {
        if let Ok(mut manager) = self.manager.lock() {
            manager.remove_connection(channel_name);
        } else {
            eprintln!("Failed to acquire channel manager lock");
        }
    }

    /// Get connection count for a specific channel
    pub fn get_connection_count(&self, channel_name: &str) -> usize {
        if let Ok(manager) = self.manager.lock() {
            manager.get_connection_count(channel_name)
        } else {
            eprintln!("Failed to acquire channel manager lock");
            0
        }
    }

    /// Clean up inactive channels
    pub fn cleanup_inactive_channels(&self) {
        if let Ok(mut manager) = self.manager.lock() {
            manager.cleanup_inactive_channels();
        } else {
            eprintln!("Failed to acquire channel manager lock");
        }
    }

    /// Get all active channels
    pub fn get_active_channels(&self) -> Vec<String> {
        if let Ok(manager) = self.manager.lock() {
            manager.get_active_channels().into_iter().cloned().collect()
        } else {
            eprintln!("Failed to acquire channel manager lock");
            Vec::new()
        }
    }
}

impl Default for ChannelIsolation {
    fn default() -> Self {
        Self::new("public".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, SenderType, MessageContent};

    #[test]
    fn test_channel_isolation_creation() {
        let isolation = ChannelIsolation::new("test-default".to_string());
        let channels = isolation.get_active_channels();
        assert!(channels.contains(&"test-default".to_string()));
    }

    #[test]
    fn test_thread_safe_operations() {
        let isolation = ChannelIsolation::default();

        // Test basic operations
        isolation.add_connection("test-channel");
        assert_eq!(isolation.get_connection_count("test-channel"), 1);

        let content = MessageContent::Question {
            text: "Thread safety test".to_string(),
            timeout_seconds: 30,
            choices: None,
        };

        let message = Message::new("test-channel".to_string(), SenderType::Agent, content);
        isolation.enqueue_message("test-channel", message);
        assert_eq!(isolation.get_queue_size("test-channel"), 1);

        let dequeued = isolation.dequeue_message("test-channel");
        assert!(dequeued.is_some());
        assert_eq!(isolation.get_queue_size("test-channel"), 0);
    }

    #[test]
    fn test_channel_isolation_between_channels() {
        let isolation = ChannelIsolation::default();

        // Add message to channel A
        let content_a = MessageContent::Question {
            text: "Channel A message".to_string(),
            timeout_seconds: 30,
            choices: None,
        };
        let message_a = Message::new("channel-a".to_string(), SenderType::Agent, content_a);
        isolation.enqueue_message("channel-a", message_a);

        // Add message to channel B
        let content_b = MessageContent::Question {
            text: "Channel B message".to_string(),
            timeout_seconds: 30,
            choices: None,
        };
        let message_b = Message::new("channel-b".to_string(), SenderType::Agent, content_b);
        isolation.enqueue_message("channel-b", message_b);

        // Verify isolation - each channel has its own message
        assert_eq!(isolation.get_queue_size("channel-a"), 1);
        assert_eq!(isolation.get_queue_size("channel-b"), 1);

        // Dequeue from channel A should not affect channel B
        let _ = isolation.dequeue_message("channel-a");
        assert_eq!(isolation.get_queue_size("channel-a"), 0);
        assert_eq!(isolation.get_queue_size("channel-b"), 1);
    }
}