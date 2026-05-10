use ailoop_core::channel::ChannelIsolation;
use ailoop_core::models::Configuration;
use ailoop_core::server::TaskStorage;
use std::sync::Arc;

use crate::server::broadcast::BroadcastManager;
use crate::server::history::MessageHistory;
use crate::server::providers::PendingPromptRegistry;

/// Shared application state. Construct once; clone (cheap — all fields are `Arc<T>`) for concurrent use.
///
/// This is the canonical state type passed to `router()` and `spawn_background_tasks()`.
/// It is also the Axum extractor state bound by `router()`.
#[derive(Clone)]
pub struct AiloopAppState {
    pub channel_manager: Arc<ChannelIsolation>,
    pub message_history: Arc<MessageHistory>,
    pub broadcast_manager: Arc<BroadcastManager>,
    pub task_storage: Arc<TaskStorage>,
    pub pending_prompt_registry: Arc<PendingPromptRegistry>,
    pub default_channel: String,
    /// Whether to serve the embedded web UI. Set by `router()` from `ServeConfig.web`.
    pub web: bool,
    /// Optional provider configuration (e.g. Telegram). Not part of the public schema but
    /// accessible within the crate for `spawn_background_tasks`.
    pub(crate) provider_config: Option<Configuration>,
}

impl AiloopAppState {
    /// Construct state with default sub-managers.
    pub fn new(default_channel: impl Into<String>) -> Self {
        let dc = default_channel.into();
        Self {
            channel_manager: Arc::new(ChannelIsolation::new(dc.clone())),
            message_history: Arc::new(MessageHistory::new()),
            broadcast_manager: Arc::new(BroadcastManager::new()),
            task_storage: Arc::new(TaskStorage::new()),
            pending_prompt_registry: Arc::new(PendingPromptRegistry::new()),
            default_channel: dc,
            web: false,
            provider_config: None,
        }
    }

    /// Attach a provider configuration (Telegram settings, etc.).
    pub fn with_provider_config(mut self, config: Configuration) -> Self {
        self.provider_config = Some(config);
        self
    }
}
