# ailoop-core

Shared Rust library for ailoop: message and workflow models, channel routing and history, HTTP/WebSocket server plumbing, transports, and agent-output parsing. Consumed by `ailoop-cli` and aligned with the SDK-facing server contract.

## Responsibilities

- Typed message and workflow models
- Channel routing and message history
- HTTP/WebSocket API implementation (with `server` feature)
- Transport abstractions (`websocket`, `file`)
- Parsing raw agent output into `AgentEvent` values

## Parser module

`parser` normalizes agent streams into `AgentEvent` values.

Registered parser kinds include:

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

## Related crates

- `../ailoop-cli`: `ailoop` binary and command handlers
- Workspace root `Cargo.toml`: shared dependency versions

## Contributing

Build, test matrix, and repository layout: [CONTRIBUTING.md](../CONTRIBUTING.md). High-level design: [ARCHITECTURE.md](../ARCHITECTURE.md).
