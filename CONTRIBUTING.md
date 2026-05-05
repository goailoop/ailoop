# Contributing to ailoop

This repository is a multi-language workspace (Rust CLI/core, TypeScript SDK, Python SDK). Keep changes focused, tested, and scoped to the modules you touch.

## Repository layout

- `ailoop-core`: shared Rust crate (protocol, models, channels, transports, server handlers)
- `ailoop-cli`: Rust CLI binary (`ailoop`) and user-facing command surface
- `ailoop-js`: TypeScript SDK for Node.js
- `ailoop-py`: async Python SDK
- `examples/web-ui`: optional static monitor used with `ailoop serve --web`
- `k8s`: deployment examples and manifests

## Architecture and system design

The system is intentionally split so protocol behavior stays centralized and consistent across interfaces:

- `ailoop-core` owns the message model, channel/history behavior, transport parsing, and server primitives.
- `ailoop-cli` is a thin edge over core behavior and exposes operational commands (`ask`, `authorize`, `say`, `navigate`, `image`, `serve`, `forward`, `config`, `workflow`, `task`, `provider`).
- SDKs (`ailoop-js`, `ailoop-py`) call the same server contracts for interoperable behavior.
- Runtime uses a single-port server model: HTTP API and WebSocket both on `:8080` by default.

Message lifecycle:

1. A caller sends an interaction (`ask`, `authorize`, `say`, `navigate`) to the server.
2. The server validates and routes by channel, then stores/broadcasts.
3. A human endpoint (terminal, web UI, or provider) responds when needed.
4. The response is correlated back to the waiting caller.

Use `ARCHITECTURE.md` for high-level design intent and contracts. Keep this file focused on contributor execution.

## Prerequisites

- Rust toolchain (stable)
- Node.js 16+
- Python 3.11+

## Clone and compile

```bash
git clone https://github.com/goailoop/ailoop.git
cd ailoop
cargo build --workspace
```

Compile distributable binary:

```bash
cargo build --release -p ailoop-cli
```

Compile SDKs:

```bash
cd ailoop-js
npm install
npm run build
cd ..

cd ailoop-py
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
python -m build
```

## Test matrix

Run only what is relevant to your changes, then run cross-cutting checks before opening a PR.

### Rust (`ailoop-core`, `ailoop-cli`)

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

### TypeScript (`ailoop-js`)

```bash
cd ailoop-js
npm install
npm run build
npm run lint
npm run type-check
npm test
```

### Python (`ailoop-py`)

```bash
cd ailoop-py
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
python -m build
ruff check .
black --check .
pytest
```

## Coding rules

- Keep implementations direct and modular.
- Avoid duplicate logic across packages.
- Do not introduce unrelated refactors in the same PR.
- Update docs for behavior or CLI/API changes.

## Pull requests

1. Create a topic branch from `main`.
2. Keep PRs small and reviewable.
3. Include:
   - what changed
   - why it changed
   - how it was tested
4. Link related issues when applicable.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/), for example:

- `feat(cli): add default deny behavior on timeout`
- `fix(core): handle websocket reconnect backoff`
- `docs(readme): update quick start for server mode`

## Git hooks (optional)

Pre-commit and related hook setup is documented in [`docs/GIT_HOOKS.md`](docs/GIT_HOOKS.md).

## Documentation updates

When changing user-visible behavior, update related docs in the same PR:

- `README.md` (end-user usage only)
- `ARCHITECTURE.md` (design-level changes)
- `CONTRIBUTING.md` (developer flow, architecture summary, build/test instructions)
- module docs like `ailoop-js/README.md`, `ailoop-py/README.md`, `ailoop-core/README.md`, `k8s/README.md`
- CLI help text in Rust (`ailoop-cli`) when commands or defaults change (`workflow`, `task`, `image`, etc.)
