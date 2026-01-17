//! Channel lifecycle management

use crate::models::Message;
use crate::server::MessageQueue;
use std::collections::HashMap;

/// Channel manager for handling multiple communication channels
pub struct ChannelManager {
    channels: HashMap<String, ChannelState>,
    default_channel: String,
}

pub(crate) struct ChannelState {
    queue: MessageQueue,
    active_connections: usize,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new(default_channel: String) -> Self {
        let mut channels = HashMap::new();

        // Create default channel
        channels.insert(
            default_channel.clone(),
            ChannelState {
                queue: MessageQueue::default(),
                active_connections: 0,
                created_at: chrono::Utc::now(),
            },
        );

        Self {
            channels,
            default_channel,
        }
    }

    /// Get or create a channel
    pub fn get_or_create_channel(&mut self, channel_name: &str) -> &mut ChannelState {
        self.channels
            .entry(channel_name.to_string())
            .or_insert_with(|| {
                println!("Creating new channel: {}", channel_name);
                ChannelState {
                    queue: MessageQueue::default(),
                    active_connections: 0,
                    created_at: chrono::Utc::now(),
                }
            })
    }

    /// Add a message to a channel
    pub fn enqueue_message(&mut self, channel_name: &str, message: Message) {
        let channel = self.get_or_create_channel(channel_name);
        channel.queue.enqueue(message);
        // Message queued (logged silently to avoid interfering with prompts)
    }

    /// Get the next message from a channel
    pub fn dequeue_message(&mut self, channel_name: &str) -> Option<Message> {
        if let Some(channel) = self.channels.get_mut(channel_name) {
            channel.queue.dequeue()
        } else {
            None
        }
    }

    /// Get queue size for a channel
    pub fn get_queue_size(&self, channel_name: &str) -> usize {
        self.channels
            .get(channel_name)
            .map(|channel| channel.queue.len())
            .unwrap_or(0)
    }

    /// Add a connection to a channel
    pub fn add_connection(&mut self, channel_name: &str) {
        let channel = self.get_or_create_channel(channel_name);
        channel.active_connections += 1;
    }

    /// Remove a connection from a channel
    pub fn remove_connection(&mut self, channel_name: &str) {
        if let Some(channel) = self.channels.get_mut(channel_name) {
            if channel.active_connections > 0 {
                channel.active_connections -= 1;
            }
        }
    }

    /// Get active connection count for a channel
    pub fn get_connection_count(&self, channel_name: &str) -> usize {
        self.channels
            .get(channel_name)
            .map(|channel| channel.active_connections)
            .unwrap_or(0)
    }

    /// Get total queue size across all channels
    pub fn get_total_queue_size(&self) -> usize {
        self.channels
            .values()
            .map(|channel| channel.queue.len())
            .sum()
    }

    /// Get total active connections across all channels
    pub fn get_total_connection_count(&self) -> usize {
        self.channels
            .values()
            .map(|channel| channel.active_connections)
            .sum()
    }

    /// Clean up inactive channels (no connections and empty queue)
    pub fn cleanup_inactive_channels(&mut self) {
        let channels_to_remove: Vec<String> = self
            .channels
            .iter()
            .filter(|(name, state)| {
                name != &&self.default_channel && // Don't remove default channel
                state.active_connections == 0 &&
                state.queue.is_empty()
            })
            .map(|(name, _)| name.clone())
            .collect();

        for channel_name in channels_to_remove {
            self.channels.remove(&channel_name);
            println!("Cleaned up inactive channel: {}", channel_name);
        }
    }

    /// Get all active channels
    pub fn get_active_channels(&self) -> Vec<&String> {
        self.channels.keys().collect()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new("public".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, MessageContent, SenderType};

    #[test]
    fn test_channel_manager_creation() {
        let manager = ChannelManager::new("test-default".to_string());
        assert_eq!(manager.default_channel, "test-default");
        assert!(manager.channels.contains_key("test-default"));
    }

    #[test]
    fn test_channel_creation_on_demand() {
        let mut manager = ChannelManager::default();

        // Channel should be created when first accessed
        let channel = manager.get_or_create_channel("new-channel");
        assert_eq!(channel.active_connections, 0);
        assert!(channel.queue.is_empty());

        // Should be in channels map
        assert!(manager.channels.contains_key("new-channel"));
    }

    #[test]
    fn test_message_enqueue_dequeue() {
        let mut manager = ChannelManager::default();

        let content = MessageContent::Question {
            text: "Test question".to_string(),
            timeout_seconds: 30,
            choices: None,
        };

        let message = Message::new("test-channel".to_string(), SenderType::Agent, content);

        manager.enqueue_message("test-channel", message.clone());
        assert_eq!(manager.get_queue_size("test-channel"), 1);

        let dequeued = manager.dequeue_message("test-channel");
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().channel, message.channel);
        assert_eq!(manager.get_queue_size("test-channel"), 0);
    }

    #[test]
    fn test_connection_management() {
        let mut manager = ChannelManager::default();

        manager.add_connection("test-channel");
        assert_eq!(manager.get_connection_count("test-channel"), 1);

        manager.add_connection("test-channel");
        assert_eq!(manager.get_connection_count("test-channel"), 2);

        manager.remove_connection("test-channel");
        assert_eq!(manager.get_connection_count("test-channel"), 1);
    }

    #[test]
    fn test_channel_cleanup() {
        let mut manager = ChannelManager::default();

        // Create a non-default channel with no connections and empty queue
        let _ = manager.get_or_create_channel("temp-channel");
        assert!(manager.channels.contains_key("temp-channel"));

        // Cleanup should remove it
        manager.cleanup_inactive_channels();
        assert!(!manager.channels.contains_key("temp-channel"));

        // Default channel should remain
        assert!(manager.channels.contains_key("public"));
    }
}
