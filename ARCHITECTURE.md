# ailoop architecture

## Overview

ailoop provides human-in-the-loop control for agent workflows through a Rust server, CLI commands, and language SDKs. The design keeps transport and protocol logic in shared core code so CLI and SDK behavior stays aligned.

## Main components

### `ailoop-core` (Rust library)

- Message/domain models
- Channel and history management
- WebSocket and HTTP API server building blocks
- Transport implementations (WebSocket, file)
- Agent parser abstractions for forwarded output

### `ailoop-cli` (Rust binary)

- User-facing commands (`ask`, `authorize`, `say`, `navigate`, `serve`, `forward`, `provider`, `task`)
- Direct mode for local terminal interaction
- Server mode for remote/hybrid interaction via WebSocket
- Config bootstrap and provider wiring

### SDKs

- `ailoop-js`: TypeScript client for Node.js applications
- `ailoop-py`: Python async client

Both SDKs target the same server contracts and message model.

### Web UI and infra

- `examples/web-ui`: browser monitor for channels and message flow
- `k8s`: manifests and examples for sidecar or service deployment

## Runtime interfaces

## Server endpoints

- WebSocket: `ws://<host>:8080`
- HTTP API: `http://<host>:8081`
- Health: `GET /api/v1/health`
- Channel/stat APIs:
  - `GET /api/channels`
  - `GET /api/channels/:channel/messages`
  - `GET /api/channels/:channel/stats`
  - `GET /api/stats`

## Message flow

1. Agent or app sends a message (`ask`, `authorize`, `say`, `navigate`) to server.
2. Server validates channel and stores/broadcasts message.
3. Human client (terminal/web UI/provider) responds where applicable.
4. Response is correlated and returned to waiting caller.

## Channel model

- Channels isolate workflows and message streams.
- Each message is tagged with channel and sender type.
- History and stats are tracked per channel.

## Configuration model

- Default CLI/server config is loaded from user config path.
- Provider config (such as Telegram chat target) is kept in config file.
- Secrets stay outside config (for example bot token via environment variable).

## Security characteristics

- Intended for trusted network boundaries by default.
- Validation on channel names and message schema.
- Timeout-safe defaults for authorization (deny on timeout/interruption).
- Container deployments can run as non-root with restricted filesystem/capabilities.

## Scalability and operations

- Async runtime and WebSocket support for concurrent clients.
- Horizontal scaling at deployment layer (Kubernetes/service replicas).
- Health endpoint for liveness/readiness checks.
- Broadcast/statistics APIs support operator visibility.

## Design intent

- Keep core protocol behavior centralized in `ailoop-core`.
- Expose simple UX at CLI/SDK edges.
- Make human approval workflows explicit, auditable, and easy to integrate.
