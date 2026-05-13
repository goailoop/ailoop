//! Integration tests for GET /api/v1/pending

use ailoop_server::server::providers::PromptType;
use ailoop_server::{router, AiloopAppState, ServeConfig};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

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

#[tokio::test]
async fn pending_empty_queue_returns_zero() {
    let state = Arc::new(AiloopAppState::new("default"));
    let r: axum::Router = router(Arc::clone(&state), &default_config()).unwrap();

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/pending")
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

    assert_eq!(json["total_count"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn pending_one_entry_returns_count_one() {
    let state = Arc::new(AiloopAppState::new("default"));

    // Register a pending entry directly
    let (_rx, _completer) = state
        .pending_prompt_registry
        .register(
            Uuid::new_v4(),
            None,
            PromptType::Decision,
            "ops".to_string(),
            "Deploy to production?".to_string(),
        )
        .await;

    let r: axum::Router = router(Arc::clone(&state), &default_config()).unwrap();

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/pending")
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

    assert_eq!(json["total_count"], 1);
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], "decision");
    assert_eq!(items[0]["channel"], "ops");
    assert_eq!(items[0]["position"], 1);
    assert_eq!(items[0]["label"], "Deploy to production?");
}

#[tokio::test]
async fn pending_channel_filter_works() {
    let state = Arc::new(AiloopAppState::new("default"));

    let (_rx1, _c1) = state
        .pending_prompt_registry
        .register(
            Uuid::new_v4(),
            None,
            PromptType::Decision,
            "ops".to_string(),
            "Ops decision".to_string(),
        )
        .await;

    let (_rx2, _c2) = state
        .pending_prompt_registry
        .register(
            Uuid::new_v4(),
            None,
            PromptType::Authorization,
            "public".to_string(),
            "Public action".to_string(),
        )
        .await;

    let r: axum::Router = router(Arc::clone(&state), &default_config()).unwrap();

    // Filter to ops channel only
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/pending?channel=ops")
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
    assert_eq!(json["total_count"], 1);
    assert_eq!(json["items"][0]["channel"], "ops");

    // No filter — both entries
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/pending")
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
    assert_eq!(json["total_count"], 2);
}

#[tokio::test]
async fn pending_completion_removes_entry() {
    let state = Arc::new(AiloopAppState::new("default"));

    let (_rx, completer) = state
        .pending_prompt_registry
        .register(
            Uuid::new_v4(),
            None,
            PromptType::Authorization,
            "ops".to_string(),
            "Deploy action".to_string(),
        )
        .await;

    // Complete the entry
    completer
        .complete(ailoop_core::models::MessageContent::Response {
            answer: Some("yes".to_string()),
            response_type: ailoop_core::models::ResponseType::AuthorizationApproved,
        })
        .await;

    let r: axum::Router = router(Arc::clone(&state), &default_config()).unwrap();

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total_count"], 0);
}

#[tokio::test]
async fn pending_invalid_channel_returns_400() {
    let state = Arc::new(AiloopAppState::new("default"));
    let r: axum::Router = router(Arc::clone(&state), &default_config()).unwrap();

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/pending?channel=invalid%20channel%20name!")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn health_queue_size_reflects_pending_registry() {
    let state = Arc::new(AiloopAppState::new("default"));

    let (_rx, _completer) = state
        .pending_prompt_registry
        .register(
            Uuid::new_v4(),
            None,
            PromptType::Decision,
            "public".to_string(),
            "Test decision".to_string(),
        )
        .await;

    let r: axum::Router = router(Arc::clone(&state), &default_config()).unwrap();

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
    assert_eq!(json["queue_size"], 1);
}
