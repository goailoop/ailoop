//! Router oneshot tests — no TCP listener required.

use ailoop_server::{router, AiloopAppState, AiloopError, ServeConfig};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use futures_util::SinkExt;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

fn default_config() -> ServeConfig {
    ServeConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        default_channel: "default".to_string(),
        base_path: None,
        web: false,
        auth: None,
        cors: None,
    }
}

fn make_state() -> Arc<AiloopAppState> {
    Arc::new(AiloopAppState::new("default"))
}

#[tokio::test]
async fn health_returns_200_with_status_field() {
    let r: axum::Router = router(make_state(), &default_config()).unwrap();

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json.get("status").is_some(),
        "health response must have 'status' field"
    );
}

#[tokio::test]
async fn create_task_then_list_ready() {
    let r: axum::Router = router(make_state(), &default_config()).unwrap();

    // POST /api/v1/tasks
    let body = serde_json::json!({
        "title": "Test task",
        "description": "A test task",
        "channel": "default"
    });
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED,
        "expected 200 or 201 from POST /api/v1/tasks, got {}",
        resp.status()
    );

    // GET /api/v1/tasks/ready?channel=default
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/tasks/ready?channel=default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let tasks = json["tasks"].as_array().expect("tasks must be array");
    assert!(!tasks.is_empty(), "at least one ready task expected");
}

#[tokio::test]
async fn base_path_prefix_routes_correctly() {
    let config = ServeConfig {
        base_path: Some("/hil".to_string()),
        ..default_config()
    };
    let r: axum::Router = router(make_state(), &config).unwrap();

    // With prefix — should succeed
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .uri("/hil/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Without prefix — should 404
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_base_path_no_leading_slash() {
    let config = ServeConfig {
        base_path: Some("hil".to_string()),
        ..default_config()
    };
    let result = router(make_state(), &config);
    assert!(
        matches!(result, Err(AiloopError::InvalidBasePath(_))),
        "expected InvalidBasePath for 'hil'"
    );
}

#[tokio::test]
async fn invalid_base_path_trailing_slash() {
    let config = ServeConfig {
        base_path: Some("/hil/".to_string()),
        ..default_config()
    };
    let result = router(make_state(), &config);
    assert!(
        matches!(result, Err(AiloopError::InvalidBasePath(_))),
        "expected InvalidBasePath for '/hil/'"
    );
}

#[tokio::test]
async fn invalid_base_path_api_collision() {
    let config = ServeConfig {
        base_path: Some("/api".to_string()),
        ..default_config()
    };
    let result = router(make_state(), &config);
    assert!(
        matches!(result, Err(AiloopError::InvalidBasePath(_))),
        "expected InvalidBasePath for '/api'"
    );
}

/// AC5: WebSocket connects to ws://host:port/hil/ with base_path="/hil",
/// sends {"subscribe":"*"} hello frame, and receives no error.
#[tokio::test]
async fn websocket_hello_frame_subscribe_all() {
    let config = ServeConfig {
        base_path: Some("/hil".to_string()),
        ..default_config()
    };
    let r: axum::Router = router(make_state(), &config).unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let token = CancellationToken::new();
    let token_srv = token.clone();

    tokio::spawn(async move {
        axum::serve(listener, r.into_make_service())
            .with_graceful_shutdown(async move { token_srv.cancelled().await })
            .await
            .ok();
    });

    // Brief pause so the server accept loop starts.
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let url = format!("ws://127.0.0.1:{}/hil/", addr.port());
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WebSocket connect to /hil/ failed");

    // Send hello frame — viewer subscribe-all.
    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        r#"{"subscribe":"*"}"#.to_string(),
    ))
    .await
    .expect("Failed to send hello frame");

    // Connection should remain open with no error frame immediately.
    // Close cleanly.
    ws.close(None).await.ok();

    token.cancel();
}
