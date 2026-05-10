# ailoop-server

HTTP/WebSocket server runtime for ailoop. Provides a composable Axum router that can be embedded into any Rust service or run standalone via `ailoop-cli`.

## Embedding

```rust
use ailoop_server::{router, spawn_background_tasks, AiloopAppState, ServeConfig};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    // 1. Build shared state.
    let state = Arc::new(AiloopAppState::new("default"));

    // 2. Configure routes.
    let config = ServeConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        default_channel: "default".to_string(),
        base_path: Some("/hil".to_string()), // mount under /hil/
        web: false,
        auth: None,
        cors: None,
    };

    // 3. Build the Axum router (pure, no I/O).
    let ailoop_router = router(Arc::clone(&state), &config).expect("invalid config");

    // 4. Merge into your own router.
    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(ailoop_router);

    // 5. Spawn background tasks (message processing, provider polling).
    let token = CancellationToken::new();
    let task_handle = spawn_background_tasks(Arc::clone(&state), &config, token.clone());

    // 6. Bind and serve.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async move { token.cancelled().await })
        .await
        .unwrap();

    let _ = task_handle.await;
}
```

## Path Prefix (`base_path`)

Set `ServeConfig.base_path` to nest all ailoop routes under a prefix:

| `base_path` | Health endpoint | WebSocket |
|---|---|---|
| `None` | `GET /api/v1/health` | `ws://host/` |
| `Some("/hil")` | `GET /hil/api/v1/health` | `ws://host/hil/` |

**Rules:**
- Must start with `/`.
- Must not end with `/`.
- Must not contain `//`.
- Must not be `/api` (collides with internal REST routes).

## Authentication

Enable token-based auth by setting `ServeConfig.auth`:

```rust
use ailoop_server::AuthConfig;

let config = ServeConfig {
    auth: Some(AuthConfig {
        tokens: vec!["my-secret-token".to_string()],
    }),
    ..ServeConfig::default()
};
```

Accepted in either header:
- `Authorization: Bearer <token>`
- `X-Api-Key: <token>`

Requests without a valid token receive `401 {"error":"unauthorized"}`.

## CORS

```rust
use ailoop_server::CorsConfig;

let config = ServeConfig {
    cors: Some(CorsConfig {
        allowed_origins: vec!["https://app.example.com".to_string()],
        allow_credentials: false,
    }),
    ..ServeConfig::default()
};
```

## Cargo Features

| Feature | Default | Effect |
|---|---|---|
| `web-ui` | on | Serves embedded HTML UI at the root path |
| `telegram` | on | Enables Telegram notification/reply provider |
| `auth` | on | Compiles the Bearer/X-Api-Key middleware |
| `openapi` | off | Reserved for future codegen tooling |

Minimal build (no web UI, no Telegram, no auth middleware):

```toml
ailoop-server = { version = "...", default-features = false }
```

## Reverse Proxy Configuration

When running behind a reverse proxy (nginx, Caddy, Traefik), ensure the `Upgrade` and `Connection` headers are forwarded for WebSocket support:

**nginx example:**
```nginx
location /hil/ {
    proxy_pass http://localhost:3000/hil/;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "Upgrade";
    proxy_set_header Host $host;
    # Strip X-Forwarded-Prefix if your app reads it:
    # proxy_set_header X-Forwarded-Prefix /hil;
}
```

The server does not read `X-Forwarded-Prefix` itself — stripping or rewriting the prefix is handled by the proxy. Set `ServeConfig.base_path` to match whatever prefix the proxy exposes.

## Graceful Shutdown

Cancel the `CancellationToken` to initiate shutdown:

1. `axum::serve` stops accepting new TCP connections.
2. In-flight requests complete normally.
3. The background task loop exits at its next 100 ms poll tick.
4. `spawn_background_tasks` join handle resolves.
5. New `POST /api/v1/messages` requests after cancellation return `503 {"error":"server shutting down"}`.

## WebSocket Protocol

See [WIRE.md](WIRE.md) for the WebSocket envelope format, hello frame schema, and semver policy.
