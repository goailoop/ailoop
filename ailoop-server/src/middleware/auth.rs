//! Tower middleware that enforces `Authorization: Bearer <token>` or `X-Api-Key: <key>`.
//!
//! When the token list is empty every request passes through unchanged (auth disabled).

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// Tower layer that wraps a service with bearer/API-key authentication.
#[derive(Clone)]
pub struct AuthLayer {
    tokens: Vec<String>,
}

impl AuthLayer {
    /// Create a new auth layer.
    ///
    /// When `tokens` is empty every request passes through (auth is effectively disabled).
    pub fn new(tokens: Vec<String>) -> Self {
        Self { tokens }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            tokens: self.tokens.clone(),
        }
    }
}

/// Service produced by [`AuthLayer`].
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    tokens: Vec<String>,
}

impl<S> Service<Request<Body>> for AuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let tokens = self.tokens.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Empty token list → auth disabled, pass through.
            if tokens.is_empty() {
                return inner.call(req).await;
            }

            let extracted = extract_token(req.headers());
            if let Some(tok) = extracted {
                if tokens.contains(&tok) {
                    return inner.call(req).await;
                }
            }

            Ok((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response())
        })
    }
}

fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(s) = auth.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    if let Some(key) = headers.get("X-Api-Key") {
        if let Ok(s) = key.to_str() {
            return Some(s.to_string());
        }
    }
    None
}
