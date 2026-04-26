# ailoop-core

Shared Rust library for ailoop. It contains core message models, server primitives, transport layers, and parser infrastructure used by `ailoop-cli` and related integrations.

## Responsibilities

- Strongly typed message and workflow models
- Channel routing and message history handling
- HTTP/WebSocket server API plumbing
- Transport abstractions (`websocket`, `file`)
- Agent output parsing into normalized events

## Parser module

`parser` converts raw agent output into standardized `AgentEvent` values.

Supported parser types include:

- `cursor`
- `jsonl`
- `opencode`

Example:

```rust
use ailoop_core::parser::{create_parser, InputFormat};

let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)?;
let line = r#"{"type":"text","timestamp":1700000000000,"part":{"text":"hello"}}"#;
let event = parser.parse_line(line).await?;
```

## Build and test

From repository root:

```bash
cargo build -p ailoop-core
cargo test -p ailoop-core
```

## Related crates

- `../ailoop-cli`: CLI entrypoint and command handlers
- workspace `Cargo.toml`: shared dependency versions

## Contributing

Use root guidelines in `../CONTRIBUTING.md`.
