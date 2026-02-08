# Ailoop - Human-in-the-Loop CLI Tool for AI Agent Communication

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Ailoop is a command-line tool that enables AI agents to communicate with human users through structured interactions. It supports both direct mode (for single-agent scenarios) and server mode (for multi-agent environments), providing a seamless bridge between AI automation and human oversight.

## Features

### Human-in-the-Loop Interactions
- Ask questions and wait for human responses
- Request authorization for critical actions
- Send notifications with priority levels
- Display images and navigate to URLs
- Channel-based isolation for multi-agent workflows

### Server Mode
- Centralized server for multi-agent environments
- WebSocket-based communication
- Channel management and message history
- Web interface for remote monitoring

### Agent Message Forwarding
- Stream agent output to centralized server
- Support for multiple agent types (Cursor, JSONL, etc.)
- Real-time message broadcasting
- Message history and channel switching

### SDKs for Application Integration
- **Python SDK**: Integrate ailoop into Python applications
- **TypeScript SDK**: Integrate ailoop into Node.js/TypeScript applications

## Installation

### Option 1: Release Binaries (Recommended)
Download the latest release from [GitHub Releases](https://github.com/goailoop/ailoop/releases):
- Linux GNU: `ailoop-x86_64-unknown-linux-gnu.tar.gz`
- Linux MUSL: `ailoop-x86_64-unknown-linux-musl.tar.gz`
- Windows: `ailoop-x86_64-pc-windows-msvc.zip`

Extract and add to your PATH, then make executable on Unix systems.

### Option 2: Homebrew (Linux)
```bash
brew install goailoop/cli/ailoop
```

### Option 3: Scoop (Windows)
```powershell
scoop bucket add goailoop https://github.com/goailoop/scoop
scoop install ailoop
```

### Verify Installation
```bash
ailoop --version
```

## SDK Integration

Ailoop provides SDKs for seamless integration into your applications:

### Python SDK

```bash
pip install ailoop-py
```

```python
from ailoop import AiloopClient

client = AiloopClient(base_url='http://localhost:8081')

# Ask a question
response = await client.ask('general', 'What is the answer?')
print(f"Response: {response.content.answer}")

# Send notification
await client.say('general', 'Task completed', 'normal')
```

[Python SDK Documentation](ailoop-py/README.md)

### TypeScript SDK

```bash
npm install ailoop-js
```

```typescript
import { AiloopClient } from 'ailoop-js';

const client = new AiloopClient({
  baseURL: 'http://localhost:8081'
});

// Ask a question
const response = await client.ask('general', 'What is the answer?');
console.log(`Response: ${response.content.answer}`);

// Send notification
await client.say('general', 'Task completed', 'normal');
```

[TypeScript SDK Documentation](ailoop-js/README.md)

## Quick Start

### Ask a Question
```bash
ailoop ask "What is the best approach for this task?"
ailoop ask "Should we proceed?" --timeout 60
ailoop ask "Review this code change" --channel dev-review
```

### Request Authorization
```bash
ailoop authorize "Deploy version 1.2.3 to production"
ailoop authorize "Delete user data" --timeout 300 --channel admin-ops
```

**Note:** If no response is received within the timeout period, authorization defaults to **DENIED** for security.

### Send Notifications
```bash
ailoop say "Build completed successfully"
ailoop say "System alert: High CPU usage detected" --priority high
ailoop say "Critical error occurred" --priority urgent --channel alerts
```

### Forward Agent Messages
```bash
# Stream agent output to server
ailoop forward --channel my-agent

# Forward from file
ailoop forward --input agent-output.jsonl --channel my-agent
```

### Server Mode
```bash
# Start server on default port (8080)
ailoop serve

# Start on custom port
ailoop serve --port 9000

# Start with custom host and channel
ailoop serve --host 0.0.0.0 --port 8080 --channel default
```

## Commands

### `ailoop ask` - Ask Questions
Ask a question and wait for a human response.

```bash
ailoop ask "What is your name?"
ailoop ask "Should we proceed?" --timeout 60
ailoop ask "Review this code" --channel dev-review --json
```

Options: `--timeout <seconds>`, `--channel <name>`, `--server <url>`, `--json`

### `ailoop authorize` - Request Authorization
Request human approval for critical actions.

```bash
ailoop authorize "Deploy to production"
ailoop authorize "Delete user data" --timeout 300 --channel admin-ops
```

Options: `--timeout <seconds>`, `--channel <name>`, `--server <url>`, `--json`

### `ailoop say` - Send Notifications
Send notification messages to human users.

```bash
ailoop say "Build completed successfully"
ailoop say "Alert: High CPU" --priority high --channel monitoring
```

Options: `--priority <low|normal|high|urgent>`, `--channel <name>`, `--server <url>`

### `ailoop serve` - Start Server
Run ailoop in server mode for multi-agent environments.

```bash
ailoop serve
ailoop serve --port 9000 --host 0.0.0.0
```

Options: `--port <number>`, `--host <address>`, `--channel <name>`

### `ailoop forward` - Forward Agent Messages
Stream agent output to centralized server.

```bash
ailoop forward --channel my-agent
ailoop forward --input output.jsonl --channel my-agent --server http://localhost:8080
```

Example: Cursor CLI
Terminal 1: `ailoop serve` (default WebSocket on 127.0.0.1:8080)
Terminal 2: `agent -p --output-format stream-json "Your prompt" 2>&1 | ailoop forward --channel public --agent-type cursor`
Optional: same from a file with `--input output.jsonl` and no pipe.
Note: forward defaults to `--url ws://127.0.0.1:8080`; use `--url` if the server runs on another host or port.

Options: `--channel <name>`, `--input <file>`, `--url <url>`, `--agent-type <type>`, `--format <stream-json|json|text>`

### `ailoop config` - Configuration
Set up your configuration interactively.

```bash
ailoop config --init
ailoop config --init --config-file ~/.config/ailoop/custom.toml
```

### `ailoop provider` - Provider status and test
List configured providers and send a test message to Telegram.

```bash
ailoop provider list
ailoop provider telegram test
```

Options: `--config-file <path>` for both; list shows name, enabled, status (no secrets).

## Telegram provider setup

Receive server messages (questions, authorizations, notifications) in Telegram and reply from your phone or desktop.

1. **Create a bot**: Talk to [@BotFather](https://t.me/BotFather), send `/newbot`, follow prompts, copy the bot token.
2. **Start a chat with your bot** (required before the bot can send you messages): In Telegram, search for **your bot** by its username (the one BotFather gave you, e.g. `@YourBot_bot`). Open that chat (do **not** use the BotFather chat). Tap **Start** or type `/start` in the message box and send it. You must do this in the chat with **your** bot; sending `/start` to @BotFather only shows BotFather's menu and does not enable your bot to message you.
3. **Get chat ID**: Message [@userinfobot](https://t.me/userinfobot) or add the bot to a group and use the group ID. The chat ID is a numeric value (e.g. `123456789`).
4. **Set token in environment** (never in config): `export AILOOP_TELEGRAM_BOT_TOKEN=your_bot_token`
5. **Configure**: Run `ailoop config --init` and enable Telegram when prompted; enter your chat ID. Or add to your config file:
   ```toml
   [providers.telegram]
   enabled = true
   chat_id = "123456789"
   ```
6. **Test**: `ailoop provider telegram test` (message should appear in the chat with your bot; exit 0 on success).
7. **Start server**: `ailoop serve`; broadcast messages will be sent to Telegram. Reply in the same chat; first response (terminal or Telegram) wins.

End-to-end example: start server with Telegram enabled and token/chat_id set; in another terminal run `ailoop ask "Approve deploy?" --server http://localhost:8080`. The question appears in Telegram; reply there or in the terminal; the ask command receives the response.

### Testing other message types (with server and Telegram)

With `ailoop serve` running and Telegram configured, use a second terminal:

- **Question** (reply as free text or yes/no):
  `ailoop ask "Approve deploy?" --server http://localhost:8080`
  Reply in Telegram with e.g. "Y", "yes", "no", or any text; the ask command prints the response.

- **Authorization** (approve or deny):
  `ailoop authorize "Deploy to production" --server http://localhost:8080`
  Reply in Telegram with "y"/"yes"/"ok" to approve or "n"/"no"/"deny" to deny.

- **Notification** (no reply expected):
  `ailoop say "Build completed" --server http://localhost:8080`
  The message appears in Telegram and in the server log.

- **Navigate** (approve or deny opening a URL):
  `ailoop navigate https://example.com --server http://localhost:8080`
  Reply in Telegram with "y" to approve or "n" to deny.

## Use Cases

### Code Review Workflow
```bash
# Agent asks for code review
ailoop ask "Please review PR #123" --channel code-review --timeout 3600

# Agent requests approval
ailoop authorize "Merge PR #123 to main branch" --channel code-review
```

### Deployment Workflow
```bash
# Start server for deployment team
ailoop serve --port 8080 --channel deployments

# In another terminal, request deployment approval
ailoop authorize "Deploy v2.1.0 to production" \
  --server http://localhost:8080 \
  --channel deployments \
  --timeout 600
```

### Agent Monitoring
```bash
# Forward agent output to server
tail -f agent.log | ailoop forward --channel my-agent

# Monitor server logs
ailoop serve
```

### Monitoring and Alerts
```bash
# Send urgent alert
ailoop say "Database connection pool exhausted!" \
  --priority urgent \
  --channel monitoring

# Show monitoring dashboard
ailoop navigate https://monitoring.example.com/dashboard \
  --channel monitoring
```

## Channels

Channels provide isolation between different workflows or agent instances. Each channel maintains its own message queue and connections.

- Channel names must be 1-64 characters
- Use lowercase alphanumeric characters, hyphens, or underscores
- Must start with a letter or digit
- Default channel is `public`

Examples of valid channel names:
- `production`
- `dev-review`
- `analytics_team`
- `channel-123`

## Troubleshooting

Connection refused: When running `ailoop forward` (or ask/authorize/say with `--server`), ensure the ailoop server is running. Start with `ailoop serve`; default WebSocket URL is `ws://127.0.0.1:8080`. Use `--url` (forward) or `--server` (other commands) if the server is on a different host or port.

Messages not appearing on server: If you run the server inside an IDE or integrated terminal, stdout may be fully buffered and you may not see "Processing message" or notification lines. Run `ailoop serve` in an external terminal (real TTY) to see output as messages are processed.

Testing message delivery to the server: To verify that the server receives and displays forwarded messages without running an agent, you can send Message JSON over WebSocket using [websocat](https://github.com/vi/websocat). First capture messages to a file (e.g. run your agent with forward using `--transport file --output /tmp/ailoop-messages.jsonl`). Start the server, then replay: `while IFS= read -r line; do echo "$line" | websocat ws://127.0.0.1:8080; done < /tmp/ailoop-messages.jsonl`. Install websocat from your package manager or GitHub releases; this is for manual testing only.

## Getting Help

- Version: `ailoop --version`
- Command help: `ailoop <command> --help`
- Documentation: [GitHub Repository](https://github.com/goailoop/ailoop)

## License

Apache License, Version 2.0 - See [LICENSE](LICENSE) file for details.

Need help? Open an issue on [GitHub](https://github.com/goailoop/ailoop/issues).
