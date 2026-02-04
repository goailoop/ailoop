//! Unit tests for PendingPromptRegistry and provider behavior

use ailoop_core::models::{MessageContent, ResponseType};
use ailoop_core::server::providers::{
    PendingPromptRegistry, PromptType, RecvTimeoutError, DEFAULT_PROMPT_TIMEOUT_SECS,
};
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_default_prompt_timeout_constant() {
    assert_eq!(DEFAULT_PROMPT_TIMEOUT_SECS, 300);
}

#[tokio::test]
async fn test_register_returns_timeout_duration() {
    let registry = PendingPromptRegistry::new();
    let (rx, _completer, timeout) = registry
        .register(Uuid::new_v4(), None, PromptType::Question)
        .await;
    assert_eq!(timeout, Duration::from_secs(DEFAULT_PROMPT_TIMEOUT_SECS));
    drop(rx);
}

#[tokio::test]
async fn test_submit_reply_oldest_first() {
    let registry = PendingPromptRegistry::new();
    let (rx, _completer, timeout) = registry
        .register(Uuid::new_v4(), None, PromptType::Question)
        .await;
    let matched = registry
        .submit_reply(None, Some("answer".into()), ResponseType::Text)
        .await;
    assert!(matched);
    let content = PendingPromptRegistry::recv_with_timeout(rx, timeout)
        .await
        .unwrap();
    assert!(matches!(content, MessageContent::Response { .. }));
    if let MessageContent::Response { answer, .. } = content {
        assert_eq!(answer.as_deref(), Some("answer"));
    }
}

#[tokio::test]
async fn test_submit_reply_by_reply_to_message_id() {
    let registry = PendingPromptRegistry::new();
    let reply_to_id = "12345".to_string();
    let (rx, _completer, timeout) = registry
        .register(
            Uuid::new_v4(),
            Some(reply_to_id.clone()),
            PromptType::Question,
        )
        .await;
    let matched = registry
        .submit_reply(Some(reply_to_id), Some("reply".into()), ResponseType::Text)
        .await;
    assert!(matched);
    let content = PendingPromptRegistry::recv_with_timeout(rx, timeout)
        .await
        .unwrap();
    if let MessageContent::Response { answer, .. } = content {
        assert_eq!(answer.as_deref(), Some("reply"));
    }
}

#[tokio::test]
async fn test_recv_timeout() {
    let registry = PendingPromptRegistry::new();
    let (rx, _completer, _timeout) = registry
        .register(Uuid::new_v4(), None, PromptType::Question)
        .await;
    let result = PendingPromptRegistry::recv_with_timeout(rx, Duration::from_millis(10)).await;
    assert!(matches!(result, Err(RecvTimeoutError::Timeout)));
}
