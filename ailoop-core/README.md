# ailoop-core

Core library for ailoop, providing shared functionality for the ailoop ecosystem.

## Overview

ailoop-core contains the foundational components used across the ailoop project:
- **Models**: Core data structures for messages, channels, workflows, etc.
- **Transport**: Client and server communication primitives (WebSocket, file-based)
- **Server & Channel**: Server management and pub/sub channel implementation
- **Services**: Reusable service components
- **Workflow**: Workflow engine for orchestrating complex agent interactions
- **Parser**: Extensible agent event parser system

## Parser Module

The `parser` module provides an extensible system for converting agent output into standardized `AgentEvent` structures.

### Supported Agent Types

- **cursor**: Parser for Cursor CLI output formats
- **jsonl**: Generic JSONL parser for any agent output with agent_type tags
- **opencode**: Parser for OpenCode stream JSON output

### Usage Example

```rust
use ailoop_core::parser::{create_parser, InputFormat};

// Create a parser for a specific agent type
let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)?;

// Parse a line of agent output
let line = r#"{"type":"text","timestamp":1700000000000,"part":{"text":"Hello"}}"#;
let event = parser.parse_line(line).await?;

if let Some(event) = event {
    println!("Parsed event type: {:?}", event.event_type);
}
```

### Key Types

- `AgentEvent`: Unified event structure with type, content, and metadata
- `EventType`: Event classification (System, User, Assistant, ToolCall, Result, Error, Custom)
- `InputFormat`: Supported input formats (Json, StreamJson, Text)
- `AgentParser`: Trait for parser implementations
- `create_parser`: Factory function to create parser instances

## Features

- Async/await support with tokio
- Extensible parser architecture
- Type-safe event structures
- Metadata tracking for sessions and requests

## Dependencies

The library uses the following main dependencies:
- `tokio`: Async runtime
- `serde`/`serde_json`: Serialization
- `anyhow`: Error handling
- `async-trait`: Async trait support
- `chrono`: Date/time handling

## Documentation

See the module-level documentation in `src/lib.rs` for detailed API documentation.
