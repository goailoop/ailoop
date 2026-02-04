//! Integration test: broadcast invokes registered notification sinks; delivery failure is logged.

use ailoop_core::models::{Message, MessageContent, SenderType};
use ailoop_core::server::broadcast::BroadcastManager;
use ailoop_core::server::providers::NotificationSink;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock sink that records received messages.
struct MockSink {
    name: &'static str,
    received: Arc<RwLock<Vec<Message>>>,
}

impl MockSink {
    fn new(name: &'static str) -> (Self, Arc<RwLock<Vec<Message>>>) {
        let received = Arc::new(RwLock::new(Vec::new()));
        let sink = Self {
            name,
            received: Arc::clone(&received),
        };
        (sink, received)
    }
}

#[async_trait]
impl NotificationSink for MockSink {
    fn name(&self) -> &str {
        self.name
    }

    async fn send(
        &self,
        message: &Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.received.write().await.push(message.clone());
        Ok(())
    }
}

/// Failing sink for delivery-failure path (broadcast still completes; error is logged).
struct FailingSink;

#[async_trait]
impl NotificationSink for FailingSink {
    fn name(&self) -> &str {
        "failing"
    }

    async fn send(
        &self,
        _message: &Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("delivery failed".into())
    }
}

#[tokio::test]
async fn test_broadcast_invokes_registered_sink() {
    let manager = BroadcastManager::new();
    let (mock, received) = MockSink::new("mock");
    manager.add_notification_sink(Arc::new(mock)).await;

    let message = Message::new(
        "test-channel".to_string(),
        SenderType::Agent,
        MessageContent::Notification {
            text: "test notification".to_string(),
            priority: ailoop_core::models::NotificationPriority::Normal,
        },
    );
    manager.broadcast_message(&message).await;

    let messages = received.read().await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].channel, "test-channel");
    if let MessageContent::Notification { text, .. } = &messages[0].content {
        assert_eq!(text, "test notification");
    } else {
        panic!("expected Notification content");
    }
}

#[tokio::test]
async fn test_broadcast_with_failing_sink_completes() {
    let manager = BroadcastManager::new();
    let (mock, received) = MockSink::new("ok");
    manager.add_notification_sink(Arc::new(mock)).await;
    manager.add_notification_sink(Arc::new(FailingSink)).await;

    let message = Message::new(
        "ch".to_string(),
        SenderType::Agent,
        MessageContent::Notification {
            text: "hi".to_string(),
            priority: ailoop_core::models::NotificationPriority::Normal,
        },
    );
    manager.broadcast_message(&message).await;

    let messages = received.read().await;
    assert_eq!(messages.len(), 1, "successful sink still receives message");
}
