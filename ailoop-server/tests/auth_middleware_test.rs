//! Auth middleware tests — on/off behaviour for REST and WS upgrade.

use ailoop_server::{router, AiloopAppState, AuthConfig, ServeConfig};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;

fn state() -> Arc<AiloopAppState> {
    Arc::new(AiloopAppState::new("default"))
}

fn config_no_auth() -> ServeConfig {
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

fn config_with_auth(tokens: Vec<&str>) -> ServeConfig {
    ServeConfig {
        auth: Some(AuthConfig {
            tokens: tokens.into_iter().map(String::from).collect(),
        }),
        ..config_no_auth()
    }
}

#[tokio::test]
async fn auth_off_all_requests_pass() {
    let r: axum::Router = router(state(), &config_no_auth()).unwrap();
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
}

#[tokio::test]
async fn auth_on_no_token_returns_401() {
    let r: axum::Router = router(state(), &config_with_auth(vec!["secret"])).unwrap();
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "unauthorized");
}

#[tokio::test]
async fn auth_on_valid_bearer_returns_200() {
    let r: axum::Router = router(state(), &config_with_auth(vec!["mysecret"])).unwrap();
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header("Authorization", "Bearer mysecret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_on_valid_x_api_key_returns_200() {
    let r: axum::Router = router(state(), &config_with_auth(vec!["apikey123"])).unwrap();
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header("X-Api-Key", "apikey123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_on_wrong_token_returns_401() {
    let r: axum::Router = router(state(), &config_with_auth(vec!["correct"])).unwrap();
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header("Authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_on_ws_upgrade_without_token_returns_401() {
    let r: axum::Router = router(state(), &config_with_auth(vec!["secret"])).unwrap();
    // A WS upgrade request without a token should be rejected before the handshake.
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header("Sec-WebSocket-Version", "13")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
