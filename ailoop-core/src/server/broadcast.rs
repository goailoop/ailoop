//! Broadcast manager for WebSocket viewer connections and notification sinks

use crate::models::{Message, MessageContent};
use crate::server::providers::NotificationSink;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use uuid::Uuid;

/// Connection type for WebSocket clients
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    Agent,
    Viewer,
}

/// Viewer connection information
#[derive(Debug, Clone)]
pub struct ViewerConnection {
    pub id: Uuid,
    pub connection_type: ConnectionType,
    pub subscribed_channels: HashSet<String>,
    pub sender: tokio::sync::mpsc::UnboundedSender<WsMessage>,
}

/// Broadcast manager for handling viewer connections and message distribution
#[derive(Clone)]
pub struct BroadcastManager {
    /// Active viewer connections: connection_id -> ViewerConnection
    viewers: Arc<RwLock<HashMap<Uuid, ViewerConnection>>>,
    /// Channel subscriptions: channel -> set of connection_ids
    channel_subscriptions: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,
    /// Notification sinks (e.g. Telegram)
    notification_sinks: Arc<RwLock<Vec<Arc<dyn NotificationSink>>>>,
}

impl BroadcastManager {
    /// Create a new broadcast manager
    pub fn new() -> Self {
        Self {
            viewers: Arc::new(RwLock::new(HashMap::new())),
            channel_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            notification_sinks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a notification sink (e.g. Telegram). Failures do not block other delivery.
    pub async fn add_notification_sink(&self, sink: Arc<dyn NotificationSink>) {
        self.notification_sinks.write().await.push(sink);
    }

    /// Add a new viewer connection
    pub async fn add_viewer(
        &self,
        connection_type: ConnectionType,
        sender: tokio::sync::mpsc::UnboundedSender<WsMessage>,
    ) -> Uuid {
        let connection_id = Uuid::new_v4();
        let viewer = ViewerConnection {
            id: connection_id,
            connection_type,
            subscribed_channels: HashSet::new(),
            sender,
        };

        self.viewers.write().await.insert(connection_id, viewer);
        connection_id
    }

    /// Remove a viewer connection
    pub async fn remove_viewer(&self, connection_id: &Uuid) {
        // Get the viewer before removing
        if let Some(viewer) = self.viewers.write().await.remove(connection_id) {
            // Remove from all channel subscriptions
            let mut channel_subs = self.channel_subscriptions.write().await;
            for channel in &viewer.subscribed_channels {
                if let Some(subscribers) = channel_subs.get_mut(channel) {
                    subscribers.remove(connection_id);
                    // Clean up empty channel subscriptions
                    if subscribers.is_empty() {
                        channel_subs.remove(channel);
                    }
                }
            }
        }
    }

    /// Subscribe a viewer to a channel
    pub async fn subscribe_to_channel(
        &self,
        connection_id: &Uuid,
        channel: &str,
    ) -> Result<(), String> {
        let mut viewers = self.viewers.write().await;
        let viewer = viewers
            .get_mut(connection_id)
            .ok_or_else(|| format!("Viewer {} not found", connection_id))?;

        viewer.subscribed_channels.insert(channel.to_string());

        // Add to channel subscriptions
        let mut channel_subs = self.channel_subscriptions.write().await;
        channel_subs
            .entry(channel.to_string())
            .or_insert_with(HashSet::new)
            .insert(*connection_id);

        Ok(())
    }

    /// Unsubscribe a viewer from a channel
    pub async fn unsubscribe_from_channel(
        &self,
        connection_id: &Uuid,
        channel: &str,
    ) -> Result<(), String> {
        let mut viewers = self.viewers.write().await;
        let viewer = viewers
            .get_mut(connection_id)
            .ok_or_else(|| format!("Viewer {} not found", connection_id))?;

        viewer.subscribed_channels.remove(channel);

        // Remove from channel subscriptions
        let mut channel_subs = self.channel_subscriptions.write().await;
        if let Some(subscribers) = channel_subs.get_mut(channel) {
            subscribers.remove(connection_id);
            // Clean up empty channel subscriptions
            if subscribers.is_empty() {
                channel_subs.remove(channel);
            }
        }

        Ok(())
    }

    /// Subscribe a viewer to all channels
    pub async fn subscribe_to_all(&self, connection_id: &Uuid) -> Result<(), String> {
        let mut viewers = self.viewers.write().await;
        let viewer = viewers
            .get_mut(connection_id)
            .ok_or_else(|| format!("Viewer {} not found", connection_id))?;

        // Get all available channels (this would come from message history)
        // For now, we'll add a special marker for "all channels"
        viewer.subscribed_channels.insert("*".to_string());

        Ok(())
    }

    /// Broadcast a message to all subscribed viewers
    pub async fn broadcast_message(&self, message: &Message) {
        self.broadcast_message_internal(message, true).await;
    }

    /// Broadcast a message to viewers only, skipping notification sinks.
    /// Used for interactive messages (Question/Authorization/Navigate) where
    /// notification sinks are called separately to get reply-to IDs.
    pub async fn broadcast_to_viewers_only(&self, message: &Message) {
        self.broadcast_message_internal(message, false).await;
    }

    /// Internal broadcast implementation with optional notification sink delivery
    async fn broadcast_message_internal(
        &self,
        message: &Message,
        include_notification_sinks: bool,
    ) {
        let channel = &message.channel;

        // Prepare JSON message
        let json_message = match serde_json::to_string(message) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Failed to serialize message for broadcast: {}", e);
                return;
            }
        };

        let ws_message = WsMessage::Text(json_message);

        // Get subscribers for this channel (and viewers subscribed to all channels)
        let mut all_subscribers = {
            let channel_subs = self.channel_subscriptions.read().await;
            channel_subs.get(channel).cloned().unwrap_or_default()
        };

        // Add viewers subscribed to all channels ("*")
        if let Some(all_channel_subs) = self.channel_subscriptions.read().await.get("*") {
            all_subscribers.extend(all_channel_subs);
        }

        // Send to all subscribers
        let viewers = self.viewers.read().await;
        for connection_id in all_subscribers {
            if let Some(viewer) = viewers.get(&connection_id) {
                if let Err(e) = viewer.sender.send(ws_message.clone()) {
                    eprintln!("Failed to send message to viewer {}: {}", connection_id, e);
                }
            }
        }

        // Send to notification sinks (e.g. Telegram). Per FR-011: log on failure.
        if include_notification_sinks {
            let sinks: Vec<Arc<dyn NotificationSink>> =
                self.notification_sinks.read().await.clone();
            let msg_type = message_content_type(message);
            for sink in sinks {
                if let Err(e) = sink.send(message).await {
                    tracing::error!(
                        provider = sink.name(),
                        message_type = %msg_type,
                        error = %e,
                        "provider delivery failed"
                    );
                }
            }
        }
    }

    /// Send message to notification sinks and return the first reply-to ID received.
    /// This is used for interactive messages (Question/Authorization/Navigate) to
    /// enable reply-to matching when users reply to Telegram messages.
    pub async fn send_to_notification_sinks_and_get_reply_to_id(
        &self,
        message: &Message,
    ) -> Option<String> {
        let sinks: Vec<Arc<dyn NotificationSink>> = self.notification_sinks.read().await.clone();

        for sink in sinks {
            match sink.send_and_get_reply_to_id(message).await {
                Ok(Some(reply_to_id)) => {
                    return Some(reply_to_id);
                }
                Ok(None) => {
                    // Sink doesn't support reply-to IDs, continue to next sink
                }
                Err(e) => {
                    tracing::error!(
                        provider = sink.name(),
                        error = %e,
                        "Failed to send message and get reply-to ID"
                    );
                }
            }
        }

        None
    }

    /// Get statistics about viewer connections
    pub async fn get_stats(&self) -> BroadcastStats {
        let viewers = self.viewers.read().await;
        let channel_subs = self.channel_subscriptions.read().await;

        let total_viewers = viewers.len();
        let agent_connections = viewers
            .values()
            .filter(|v| matches!(v.connection_type, ConnectionType::Agent))
            .count();
        let viewer_connections = total_viewers - agent_connections;
        let active_channels = channel_subs.len();

        BroadcastStats {
            total_viewers,
            agent_connections,
            viewer_connections,
            active_channels,
        }
    }

    /// Get all channels with active subscriptions
    pub async fn get_active_channels(&self) -> Vec<String> {
        let channel_subs = self.channel_subscriptions.read().await;
        channel_subs.keys().cloned().collect()
    }
}

impl Default for BroadcastManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about broadcast manager state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastStats {
    pub total_viewers: usize,
    pub agent_connections: usize,
    pub viewer_connections: usize,
    pub active_channels: usize,
}

use serde::{Deserialize, Serialize};

fn message_content_type(message: &Message) -> &'static str {
    match &message.content {
        MessageContent::Question { .. } => "question",
        MessageContent::Authorization { .. } => "authorization",
        MessageContent::Notification { .. } => "notification",
        MessageContent::Response { .. } => "response",
        MessageContent::Navigate { .. } => "navigate",
        _ => "other",
    }
}
