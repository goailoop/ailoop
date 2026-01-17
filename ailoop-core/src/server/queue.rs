//! Message queuing system

use crate::models::Message;
use std::collections::VecDeque;

/// Message queue for handling incoming messages
pub struct MessageQueue {
    queue: VecDeque<Message>,
    max_size: usize,
}

impl MessageQueue {
    /// Create a new message queue with maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
        }
    }

    /// Add a message to the queue
    pub fn enqueue(&mut self, message: Message) {
        // If queue is full, remove oldest message
        if self.queue.len() >= self.max_size {
            self.queue.pop_front();
        }
        self.queue.push_back(message);
    }

    /// Remove and return the next message from the queue
    pub fn dequeue(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get the current queue size
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Clear all messages from the queue
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new(1000) // Default max size of 1000 messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, MessageContent, SenderType};

    #[test]
    fn test_message_queue_creation() {
        let queue = MessageQueue::new(100);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_message_enqueue_dequeue() {
        let mut queue = MessageQueue::new(10);

        let content = MessageContent::Question {
            text: "Test question".to_string(),
            timeout_seconds: 30,
            choices: None,
        };

        let message = Message::new("test-channel".to_string(), SenderType::Agent, content);

        queue.enqueue(message.clone());
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().channel, message.channel);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_message_queue_max_size() {
        let mut queue = MessageQueue::new(2);

        let content = MessageContent::Question {
            text: "Test question".to_string(),
            timeout_seconds: 30,
            choices: None,
        };

        // Add 3 messages to a queue with max size 2
        for i in 0..3 {
            let mut message =
                Message::new(format!("channel-{}", i), SenderType::Agent, content.clone());
            queue.enqueue(message);
        }

        // Should only have 2 messages (oldest removed)
        assert_eq!(queue.len(), 2);

        // First dequeued should be the second message (oldest remaining)
        let first = queue.dequeue();
        assert!(first.is_some());
        assert_eq!(first.unwrap().channel, "channel-1");
    }
}
