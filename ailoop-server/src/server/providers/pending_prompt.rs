//! Pending prompt registry: match provider replies to waiting prompts

use ailoop_core::models::{Configuration, MessageContent, ResponseType};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use uuid::Uuid;

/// Default timeout in seconds for pending prompts — retained for reference, no longer used as
/// the runtime fallback in prompt dispatch.
pub const DEFAULT_PROMPT_TIMEOUT_SECS: u64 = 300;

/// Type of interactive prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptType {
    Authorization,
    Navigation,
    Decision,
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

    /// Register a pending prompt. Returns a receiver and a completer (for terminal).
    /// First response (terminal via completer or provider via submit_reply) wins.
    pub async fn register(
        &self,
        message_id: Uuid,
        reply_to_message_id: Option<String>,
        _prompt_type: PromptType,
    ) -> (oneshot::Receiver<MessageContent>, PendingPromptCompleter) {
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
        (rx, completer)
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

    /// Wait for the prompt response, optionally bounded by a timeout.
    /// Returns Ok(content) on response, Err(Timeout) on expiry, Err(Closed) if sender dropped.
    /// When timeout is None, waits indefinitely until a response arrives or the sender is dropped.
    pub async fn recv_maybe_timeout(
        rx: oneshot::Receiver<MessageContent>,
        timeout: Option<std::time::Duration>,
    ) -> Result<MessageContent, RecvTimeoutError> {
        match timeout {
            Some(d) => Self::recv_with_timeout(rx, d).await,
            None => rx.await.map_err(|_| RecvTimeoutError::Closed),
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

/// Resolve the effective timeout for a prompt, applying precedence rules.
///
/// Resolution order (highest precedence first):
/// 1. `message_timeout_secs > 0` → use that value.
/// 2. Env var `AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS` set to a positive integer → use that value.
/// 3. `config.timeout_seconds` is `Some(v)` and `v > 0` → use that value.
/// 4. Otherwise → `None` (infinite wait).
///
/// `timeout_seconds = Some(0)` in config means "infinite" (consistent with the 0 = no timeout
/// convention). The `> 3600` guard in `Configuration::validate()` already accepts `Some(0)`.
pub fn resolve_effective_timeout(
    message_timeout_secs: u32,
    config: Option<&Configuration>,
) -> Option<std::time::Duration> {
    if message_timeout_secs > 0 {
        return Some(std::time::Duration::from_secs(message_timeout_secs as u64));
    }
    if let Ok(val_str) = std::env::var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS") {
        match val_str.parse::<u64>() {
            Ok(val) if val > 0 => return Some(std::time::Duration::from_secs(val)),
            _ => {
                tracing::warn!(
                    "AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS is set but not a valid positive integer: {:?}; ignoring",
                    val_str
                );
            }
        }
    }
    if let Some(cfg) = config {
        if let Some(t) = cfg.timeout_seconds {
            if t > 0 {
                return Some(std::time::Duration::from_secs(t as u64));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::oneshot;

    fn make_config(timeout_seconds: Option<u32>) -> Configuration {
        Configuration {
            timeout_seconds,
            ..Configuration::default()
        }
    }

    // --- resolve_effective_timeout ---
    //
    // These tests touch a process-wide environment variable. Guard every timeout-resolution
    // assertion so parallel test execution cannot leak env state across tests.
    use std::sync::{Mutex, MutexGuard};
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct PromptTimeoutEnvGuard {
        previous: Option<String>,
        _guard: MutexGuard<'static, ()>,
    }

    impl PromptTimeoutEnvGuard {
        fn unset() -> Self {
            let guard = ENV_MUTEX.lock().unwrap();
            let previous = std::env::var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS").ok();
            std::env::remove_var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS");
            Self {
                previous,
                _guard: guard,
            }
        }

        fn set(value: &str) -> Self {
            let guard = ENV_MUTEX.lock().unwrap();
            let previous = std::env::var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS").ok();
            std::env::set_var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS", value);
            Self {
                previous,
                _guard: guard,
            }
        }
    }

    impl Drop for PromptTimeoutEnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS", value),
                None => std::env::remove_var("AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS"),
            }
        }
    }

    #[test]
    fn test_resolve_message_timeout_overrides_all() {
        let _guard = PromptTimeoutEnvGuard::set("120");
        let cfg = make_config(Some(180));
        let result = resolve_effective_timeout(60, Some(&cfg));
        assert_eq!(result, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_resolve_config_positive_returns_some() {
        let _guard = PromptTimeoutEnvGuard::unset();
        let cfg = make_config(Some(180));
        let result = resolve_effective_timeout(0, Some(&cfg));
        assert_eq!(result, Some(Duration::from_secs(180)));
    }

    #[test]
    fn test_resolve_config_zero_returns_none() {
        let _guard = PromptTimeoutEnvGuard::unset();
        let cfg = make_config(Some(0));
        let result = resolve_effective_timeout(0, Some(&cfg));
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_config_none_returns_none() {
        let _guard = PromptTimeoutEnvGuard::unset();
        let cfg = make_config(None);
        let result = resolve_effective_timeout(0, Some(&cfg));
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_no_config_returns_none() {
        let _guard = PromptTimeoutEnvGuard::unset();
        let result = resolve_effective_timeout(0, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_env_var_positive_overrides_config() {
        let _guard = PromptTimeoutEnvGuard::set("120");
        let cfg = make_config(Some(180));
        let result = resolve_effective_timeout(0, Some(&cfg));
        assert_eq!(result, Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_resolve_env_var_zero_ignored_config_applies() {
        let _guard = PromptTimeoutEnvGuard::set("0");
        let cfg = make_config(Some(180));
        let result = resolve_effective_timeout(0, Some(&cfg));
        assert_eq!(result, Some(Duration::from_secs(180)));
    }

    #[test]
    fn test_resolve_env_var_non_integer_ignored_config_applies() {
        let _guard = PromptTimeoutEnvGuard::set("abc");
        let cfg = make_config(Some(180));
        let result = resolve_effective_timeout(0, Some(&cfg));
        assert_eq!(result, Some(Duration::from_secs(180)));
    }

    // --- recv_maybe_timeout ---

    #[tokio::test]
    async fn test_recv_maybe_timeout_returns_value() {
        let (tx, rx) = oneshot::channel();
        let content = MessageContent::Response {
            answer: Some("hello".to_string()),
            response_type: ailoop_core::models::ResponseType::Text,
        };
        tx.send(content.clone()).unwrap();
        let result =
            PendingPromptRegistry::recv_maybe_timeout(rx, Some(Duration::from_secs(5))).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recv_maybe_timeout_finite_expires() {
        let (_tx, rx) = oneshot::channel::<MessageContent>();
        let result =
            PendingPromptRegistry::recv_maybe_timeout(rx, Some(Duration::from_millis(20))).await;
        assert!(matches!(result, Err(RecvTimeoutError::Timeout)));
    }

    #[tokio::test]
    async fn test_recv_maybe_timeout_none_closed_on_drop() {
        let (tx, rx) = oneshot::channel::<MessageContent>();
        drop(tx);
        let result = PendingPromptRegistry::recv_maybe_timeout(rx, None).await;
        assert!(matches!(result, Err(RecvTimeoutError::Closed)));
    }
}
