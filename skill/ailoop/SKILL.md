---
name: ailoop
description: Human-in-the-loop CLI for AI agent communication. Use when agents need to ask questions, request authorization, send notifications, run server mode, or forward messages; integrating ailoop SDK (Python/TypeScript) or configuring channels and providers.
license: Apache-2.0
---

# Ailoop

Ailoop is a CLI and server that lets AI agents talk to humans in a structured way: ask questions, request authorization, send notifications, and forward agent output. Use this skill when you need to add human-in-the-loop flows or integrate ailoop into an agent or app.

## When to use

- An agent must ask a human a question and wait for an answer.
- An agent needs approval before a critical action (e.g. deploy, delete).
- Sending notifications (build done, alerts) to humans or channels.
- Running a central server for multiple agents (server mode).
- Forwarding agent output to a server or channel.
- Integrating ailoop from Python or TypeScript (SDK).
- Configuring channels or providers (e.g. Telegram).
- Managing tasks with states and dependency graphs.
- Orchestrating workflows with approval gates.

## Core Concepts

### Message types

| Type | Direction | Description |
|------|-----------|-------------|
| **ask** | Agent -> Human | Ask a question, optionally with multiple choices. Blocks until answered. |
| **authorize** | Agent -> Human | Request approval. Defaults to **denied** on timeout or interruption. |
| **say** | Agent -> Human | One-way notification with priority levels. |
| **image** | Agent -> Human | Display an image (file path or URL). |
| **navigate** | Agent -> Human | Suggest user navigate to a URL. |
| **response** | Human -> Agent | Reply to a question or authorization request. |

### Channels

Channels isolate workflows. Names: 1-64 chars, lowercase alphanumeric, hyphens, underscores; must start with letter or digit. Default channel is `public`.

### Architecture

```
Agent/CLI --> HTTP REST API --> ailoop serve <-- WebSocket --> SDK clients
             POST /api/v1/messages         |
             GET  /api/v1/messages/{id}     +-- Telegram provider
             GET  /api/v1/tasks             +-- Terminal (TTY)
             ws://{host}/ws
```

The server (`ailoop serve`) is the central hub. CLI commands and SDK clients communicate with it via HTTP and WebSocket.

## Environment Variables

Variables that apply across CLI, server, and SDK clients:

| Variable | Description | Default |
|----------|-------------|---------|
| `AILOOP_SERVER` | Server URL for remote operation. Takes precedence over `--server` CLI flag. URL is auto-converted (http->ws, https->wss). | None |
| `AILOOP_TELEGRAM_BOT_TOKEN` | Telegram Bot API token. Required when Telegram provider is enabled. | None |
| `RUST_LOG` | Server logging verbosity (used by `ailoop serve`). | `ailoop=info` |
| `XDG_CONFIG_HOME` | Base config directory. Config file: `$XDG_CONFIG_HOME/ailoop/config.toml`. | `~/.config` |

The Python and TypeScript SDKs do **not** read environment variables -- all configuration is passed via constructor parameters.

For CLI-specific environment variables and full details, see [`references/ailoop-cli.md`](references/ailoop-cli.md).

## Detailed References

For complete documentation, see:

- **CLI**: [`references/ailoop-cli.md`](references/ailoop-cli.md) -- all commands, flags, task management, workflow orchestration, provider setup
- **API**: [`references/ailoop-api.md`](references/ailoop-api.md) -- REST endpoints, WebSocket protocol, message schemas, curl examples
- **Python SDK**: [`references/ailoop-py.md`](references/ailoop-py.md) -- `AiloopClient` API, WebSocket listening, models, task management
- **TypeScript SDK**: [`references/ailoop-js.md`](references/ailoop-js.md) -- `AiloopClient` API, `MessageFactory`, types, WebSocket listening

## Quick Start

### CLI

```bash
# Install
brew install goailoop/cli/ailoop

# Ask a question (blocks until answered)
ailoop ask "What is the best approach?"

# Request authorization (denied on timeout)
ailoop authorize "Deploy v1.2.3 to production" --timeout 300

# Send a notification
ailoop say "Build completed" --priority high

# Start server for multi-agent use
ailoop serve
```

### Python

```bash
pip install ailoop-py
```

```python
from ailoop import AiloopClient

async with AiloopClient(server_url="http://localhost:8080") as client:
    msg = await client.ask("Ready to proceed?", channel="dev")
    await client.say("Task completed", channel="dev", priority="normal")
```

### TypeScript

```bash
npm install ailoop-js
```

```typescript
import { AiloopClient } from 'ailoop-js';

const client = new AiloopClient({ baseURL: 'http://localhost:8080' });
const msg = await client.ask('dev', 'Ready to proceed?');
await client.say('dev', 'Task completed', 'normal');
```

## Getting help

- `ailoop --version`, `ailoop <command> --help`
- [GitHub](https://github.com/goailoop/ailoop)
