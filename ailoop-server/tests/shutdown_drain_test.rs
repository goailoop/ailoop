//! Background task shutdown drain tests.

use ailoop_server::{spawn_background_tasks, AiloopAppState, ServeConfig};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn background_tasks_exit_within_500ms_after_cancel() {
    let state = Arc::new(AiloopAppState::new("default"));
    let config = ServeConfig::default();
    let token = CancellationToken::new();

    let handle = spawn_background_tasks(Arc::clone(&state), &config, token.clone());

    // Cancel immediately — no messages queued so the loop exits at the next tick.
    token.cancel();

    let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
    assert!(
        result.is_ok(),
        "background tasks did not exit within 500 ms after cancellation"
    );
}
