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
- Interactive terminal UI with real-time monitoring
- WebSocket-based communication
- Channel management and message history
- Web interface for remote monitoring

### Agent Message Forwarding
- Stream agent output to centralized server
- Support for multiple agent types (Cursor, JSONL, etc.)
- Real-time message broadcasting
- Message history and channel switching

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

Options: `--channel <name>`, `--input <file>`, `--server <url>`, `--format <cursor|jsonl>`

### `ailoop config` - Configuration
Set up your configuration interactively.

```bash
ailoop config --init
ailoop config --init --config-file ~/.config/ailoop/custom.toml
```

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

# View in terminal UI or web interface
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

### Command Not Found
- Ensure binary is in your PATH
- Make executable on Unix: `chmod +x ailoop`

### Server won't start
- Check if the port is already in use: `lsof -i :8080` (Linux) or `netstat -an | grep 8080` (Windows)
- Ensure you have permission to bind to the specified port (ports < 1024 require root/admin)
- Check firewall settings

### Timeout issues
- Increase timeout value if operations take longer than expected
- Use `--timeout 0` to wait indefinitely (not recommended for production)

### Channel validation errors
- Ensure channel names follow the naming convention
- Channel names are case-sensitive
- Avoid special characters except hyphens and underscores

## Getting Help

- Version: `ailoop --version`
- Command help: `ailoop <command> --help`
- Documentation: [GitHub Repository](https://github.com/goailoop/ailoop)

## License

Apache License, Version 2.0 - See [LICENSE](LICENSE) file for details.

Need help? Open an issue on [GitHub](https://github.com/goailoop/ailoop/issues).
