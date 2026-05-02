//! Integration test: broadcast invokes registered notification sinks; delivery failure is logged.

use ailoop_core::models::{Message, MessageContent, SenderType};
use ailoop_core::server::broadcast::{BroadcastManager, ConnectionType};
use ailoop_core::server::providers::NotificationSink;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message as WsMessage;

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

#[tokio::test]
async fn subscribe_to_all_delivers_broadcasts_for_any_channel() {
    let manager = BroadcastManager::new();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let connection_id = manager.add_viewer(ConnectionType::Viewer, tx).await;
    manager.subscribe_to_all(&connection_id).await.unwrap();

    let message = Message::new(
        "some-channel".to_string(),
        SenderType::Agent,
        MessageContent::Notification {
            text: "live".to_string(),
            priority: ailoop_core::models::NotificationPriority::Normal,
        },
    );

    manager.broadcast_message(&message).await;

    let ws = rx
        .try_recv()
        .expect("subscribe * must register channel_subscriptions");
    match ws {
        WsMessage::Text(s) => assert!(s.contains("live")),
        other => panic!("expected Text broadcast, got {other:?}"),
    }
}
