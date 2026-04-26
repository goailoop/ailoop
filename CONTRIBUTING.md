# Contributing to ailoop

This repository is a multi-language workspace (Rust CLI/core, TypeScript SDK, Python SDK). Keep changes focused, tested, and scoped to the modules you touch.

## Development setup

## Prerequisites

- Rust toolchain (stable)
- Node.js 16+
- Python 3.11+

## Clone and build

```bash
git clone https://github.com/goailoop/ailoop.git
cd ailoop
cargo build
```

## Test matrix

Run only what is relevant to your changes, then run cross-cutting checks before opening a PR.

### Rust (`ailoop-core`, `ailoop-cli`)

```bash
cargo fmt --all
cargo test
```

### TypeScript (`ailoop-js`)

```bash
cd ailoop-js
npm install
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

## Documentation updates

When changing user-visible behavior, update related docs in the same PR:

- `README.md` (consumer usage)
- `ARCHITECTURE.md` (design-level changes)
- module docs like `ailoop-js/README.md`, `ailoop-py/README.md`, `k8s/README.md`
