//! Pending prompt registry: match provider replies to waiting prompts

use crate::models::{MessageContent, ResponseType};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use uuid::Uuid;

/// Default timeout in seconds for pending prompts when not specified by the message (per spec).
pub const DEFAULT_PROMPT_TIMEOUT_SECS: u64 = 300;

/// Type of interactive prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptType {
    Question,
    Authorization,
    Navigation,
}

/// A pending prompt awaiting response (terminal or provider).
#[derive(Debug)]
struct PendingEntry {
    entry_id: Uuid,
    message_id: Uuid,
    reply_to_message_id: Option<String>,
    _created_at: std::time::Instant,
    tx: oneshot::Sender<MessageContent>,
}

/// Completer for a single registered prompt (e.g. terminal response wins).
#[derive(Clone)]
pub struct PendingPromptCompleter {
    entry_id: Uuid,
    inner: Arc<RwLock<VecDeque<PendingEntry>>>,
}

impl PendingPromptCompleter {
    /// Complete this prompt with the given content (e.g. from terminal). Idempotent; only first
    /// call takes effect.
    pub async fn complete(&self, content: MessageContent) {
        let mut guard = self.inner.write().await;
        if let Some(pos) = guard.iter().position(|e| e.entry_id == self.entry_id) {
            let entry = guard.remove(pos).expect("position exists");
            let _ = entry.tx.send(content);
        }
    }
}

/// In-memory registry of pending prompts. Match by reply_to or oldest first.
#[derive(Clone)]
pub struct PendingPromptRegistry {
    inner: Arc<RwLock<VecDeque<PendingEntry>>>,
}

impl PendingPromptRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Register a pending prompt. Returns a receiver, a completer (for terminal), and default
    /// timeout. First response (terminal via completer or provider via submit_reply) wins.
    pub async fn register(
        &self,
        message_id: Uuid,
        reply_to_message_id: Option<String>,
        _prompt_type: PromptType,
    ) -> (
        oneshot::Receiver<MessageContent>,
        PendingPromptCompleter,
        std::time::Duration,
    ) {
        let entry_id = Uuid::new_v4();
        let (tx, rx) = oneshot::channel();
        let entry = PendingEntry {
            entry_id,
            message_id,
            reply_to_message_id,
            _created_at: std::time::Instant::now(),
            tx,
        };
        self.inner.write().await.push_back(entry);
        let completer = PendingPromptCompleter {
            entry_id,
            inner: Arc::clone(&self.inner),
        };
        let timeout = std::time::Duration::from_secs(DEFAULT_PROMPT_TIMEOUT_SECS);
        (rx, completer, timeout)
    }
}

impl PendingPromptRegistry {
    /// Wait for the prompt response with timeout. Use the receiver returned from register().
    pub async fn recv_with_timeout(
        rx: oneshot::Receiver<MessageContent>,
        timeout: std::time::Duration,
    ) -> Result<MessageContent, RecvTimeoutError> {
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(content)) => Ok(content),
            Ok(Err(_)) => Err(RecvTimeoutError::Closed),
            Err(_) => Err(RecvTimeoutError::Timeout),
        }
    }

    /// Submit a reply from a provider. Matches by reply_to_message_id or oldest first.
    /// Returns true if matched and response was sent to the waiting task.
    pub async fn submit_reply(
        &self,
        reply_to_message_id: Option<String>,
        answer: Option<String>,
        response_type: ResponseType,
    ) -> bool {
        let content = MessageContent::Response {
            answer,
            response_type,
        };
        let mut guard = self.inner.write().await;
        if let Some(reply_to) = &reply_to_message_id {
            if let Some(pos) = guard
                .iter()
                .position(|e| e.reply_to_message_id.as_deref() == Some(reply_to.as_str()))
            {
                let entry = guard.remove(pos).expect("position exists");
                let _ = entry.tx.send(content);
                return true;
            }
        }
        if let Some(entry) = guard.pop_front() {
            let _ = entry.tx.send(content);
            return true;
        }
        false
    }

    /// Submit a reply that targets a specific message ID (e.g. via HTTP API).
    /// Returns true if a pending prompt was waiting for that message.
    pub async fn submit_reply_for_message(
        &self,
        message_id: Uuid,
        answer: Option<String>,
        response_type: ResponseType,
    ) -> bool {
        let content = MessageContent::Response {
            answer,
            response_type,
        };
        let mut guard = self.inner.write().await;
        if let Some(pos) = guard.iter().position(|e| e.message_id == message_id) {
            let entry = guard.remove(pos).expect("position exists");
            let _ = entry.tx.send(content);
            return true;
        }
        false
    }
}

/// Error from recv_with_timeout
#[derive(Debug)]
pub enum RecvTimeoutError {
    Timeout,
    Closed,
}

impl Default for PendingPromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
