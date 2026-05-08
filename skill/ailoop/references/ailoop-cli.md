# ailoop CLI Reference

The `ailoop` binary is the core CLI and server for human-in-the-loop AI agent communication.

## Installation

```bash
# Homebrew (Linux)
brew install goailoop/cli/ailoop

# Scoop (Windows)
scoop bucket add goailoop https://github.com/goailoop/scoop
scoop install ailoop

# Release binaries (Linux/Windows)
# https://github.com/goailoop/ailoop/releases
```

Verify: `ailoop --version`

## Environment Variables

| Variable | Description | Default | Applies to |
|----------|-------------|---------|------------|
| `AILOOP_SERVER` | Server URL for remote operation. Overrides `--server` flag. Auto-converts http->ws, https->wss for WebSocket connections. | None | All remote commands |
| `AILOOP_TELEGRAM_BOT_TOKEN` | Telegram Bot API token. Required for Telegram provider. | None | `serve`, `provider telegram test` |
| `RUST_LOG` | Server log verbosity. Uses `tracing_subscriber` EnvFilter. | `ailoop=info` | `serve` |
| `XDG_CONFIG_HOME` | Base config directory. Config: `$XDG_CONFIG_HOME/ailoop/config.toml`. | `~/.config` | `config`, `provider` |
| `HOME` | Home directory. Used to resolve `~/` in config file paths. | System default | All commands |

### Docker / K8s deployment variables

These are set in docker-compose and K8s manifests. They are not read by the ailoop binary directly but are used as deployment patterns:

| Variable | Description | Typical Value |
|----------|-------------|---------------|
| `AILOOP_HOST` | Bind host for container | `0.0.0.0` |
| `AILOOP_PORT` | Server port for container | `8080` |
| `AILOOP_BASE_URL` | URL for SDK clients to reach the sidecar | `http://ailoop-sidecar:8081` |

## Global Options

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

Common flags on most commands:

| Flag | Description |
|------|-------------|
| `-c`, `--channel <CHANNEL>` | Channel name (default: `public`) |
| `--server <SERVER>` | Remote server URL (default: empty = local) |
| `--json` | Output in JSON format |

## ask -- Ask a question

Ask a question and wait for a human response. Blocks until answered.

```bash
ailoop ask "What is the best approach?"
ailoop ask "Choose a color|red|blue|green"
ailoop ask "Should we proceed?" --timeout 60 --channel dev-review --json
```

**Multiple choice format:** Use pipe separator: `"question|choice1|choice2|choice3"`

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Target channel |
| `-t`, `--timeout` | `0` (none) | Timeout in seconds, 0 = no timeout |
| `--server` | empty | Server URL for remote operation |
| `--json` | off | JSON output |

**JSON response format (text):**
```json
{"response": "answer text", "channel": "public", "timestamp": "..."}
```

**JSON response format (multiple choice):**
```json
{
  "response": "red",
  "channel": "public",
  "timestamp": "...",
  "metadata": {"index": 0, "value": "red"}
}
```

## authorize -- Request authorization

Request human approval. Defaults to **DENIED** on timeout, Ctrl+C, or read errors.

```bash
ailoop authorize "Deploy version 1.2.3 to production"
ailoop authorize "Delete user data" --timeout 300 --channel admin-ops
ailoop authorize "Restart service?" --default no
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Target channel |
| `-t`, `--timeout` | `300` | Timeout in seconds |
| `--server` | empty | Server URL for remote operation |
| `--json` | off | JSON output |
| `--default` | `yes` | Decision when Enter is pressed (`yes` or `no`) |

Press Enter to accept the configured default. Timeout, read errors, and Ctrl+C always resolve to denied for security.

## say -- Send a notification

Non-blocking. Sends a one-way notification.

```bash
ailoop say "Build completed successfully"
ailoop say "System alert: High CPU" --priority high --channel monitoring
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Target channel |
| `-p`, `--priority` | `normal` | `low`, `normal`, `high`, `urgent` |
| `--server` | empty | Server URL for remote operation |

## image -- Display an image

Display an image to the user.

```bash
ailoop image ./screenshot.png
ailoop image https://example.com/diagram.png --channel design
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Target channel |
| `--server` | empty | Server URL for remote operation |

## navigate -- Suggest URL navigation

Suggest the user navigate to a URL.

```bash
ailoop navigate https://dashboard.example.com/deploy/123
ailoop navigate https://docs.example.com --channel onboarding
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Target channel |
| `--server` | empty | Server URL for remote operation |

## serve -- Run server mode

Start the ailoop server for multi-agent communication. Provides both HTTP REST API and WebSocket endpoint.

```bash
ailoop serve
ailoop serve --port 9000 --host 0.0.0.0
ailoop serve --channel default-channel
```

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `127.0.0.1` | Bind address |
| `-p`, `--port` | `8080` | Server port |
| `-c`, `--channel` | `public` | Default channel |

The server exposes:
- HTTP API at `http://{host}:{port}/api/v1/...`
- WebSocket at `ws://{host}:{port}/ws`

## forward -- Stream agent output

Stream agent output to the server. Reads from stdin by default.

```bash
# Pipe agent output to server
agent -p --output-format stream-json "prompt" 2>&1 | ailoop forward --channel public --agent-type cursor

# From file
ailoop forward --input output.jsonl --channel my-agent

# Save to file instead of WebSocket
ailoop forward --transport file --output /tmp/ailoop-messages.jsonl
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Channel for messages |
| `--agent-type` | auto | `cursor`, `jsonl`, `opencode`, or auto-detect |
| `--format` | `stream-json` | `json`, `stream-json`, `text` |
| `--transport` | `websocket` | `websocket` or `file` |
| `--url` | `ws://127.0.0.1:8080` | WebSocket URL |
| `--output` | none | Output file (for file transport) |
| `--input` | stdin | Input file path |
| `--client-id` | none | Client ID for tracking |

**Cursor CLI example:**
1. Start server: `ailoop serve`
2. In another terminal: `agent -p --output-format stream-json "Your prompt" 2>&1 | ailoop forward --channel public --agent-type cursor`

## task -- Task management

Manage tasks with states and dependency tracking.

### task create

```bash
ailoop task create "Deploy service" --description "Deploy v2 to staging" --channel ops
```

| Flag | Default | Description |
|------|---------|-------------|
| `-d`, `--description` | required | Task description |
| `-c`, `--channel` | `public` | Target channel |
| `--server` | empty | Server URL |
| `--json` | off | JSON output |

### task list

```bash
ailoop task list --channel ops --state pending --json
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--channel` | `public` | Channel filter |
| `--state` | none | Filter by state (`pending`, `done`, `abandoned`) |
| `--server` | empty | Server URL |
| `--json` | off | JSON output |

### task show

```bash
ailoop task show <TASK_ID> --json
```

### task update

```bash
ailoop task update <TASK_ID> --state done --channel ops
```

| Flag | Default | Description |
|------|---------|-------------|
| `-s`, `--state` | required | New state (`pending`, `done`, `abandoned`) |
| `-c`, `--channel` | `public` | Target channel |
| `--server` | empty | Server URL |
| `--json` | off | JSON output |

### task dep -- Dependency management

```bash
# Add dependency
ailoop task dep add <CHILD_ID> <PARENT_ID> --type blocks

# Remove dependency
ailoop task dep remove <CHILD_ID> <PARENT_ID>

# Show dependency graph
ailoop task dep graph <TASK_ID>
```

Dependency types: `blocks`, `related`, `parent`.

### task ready / task blocked

```bash
ailoop task ready --channel ops --json
ailoop task blocked --channel ops --json
```

## workflow -- Workflow orchestration

Manage workflow definitions and executions.

### workflow start

```bash
ailoop workflow start <WORKFLOW_NAME> --initiator cli-user
```

### workflow status

```bash
ailoop workflow status <EXECUTION_ID> --json
```

### workflow list / list-defs

```bash
ailoop workflow list          # List running executions
ailoop workflow list-defs     # List available workflow YAML files
```

### workflow approve / deny

```bash
ailoop workflow approve <APPROVAL_ID> --operator alice
ailoop workflow deny <APPROVAL_ID> --operator alice
```

### workflow list-approvals

```bash
ailoop workflow list-approvals --execution <EXECUTION_ID> --json
```

### workflow logs

```bash
ailoop workflow logs <EXECUTION_ID> --limit 50 --follow
ailoop workflow logs <EXECUTION_ID> --state deploy
```

| Flag | Default | Description |
|------|---------|-------------|
| `-s`, `--state` | all | Filter by state name |
| `-l`, `--limit` | `100` | Number of recent lines |
| `--offset` | `0` | Skip first N lines |
| `-f`, `--follow` | off | Follow output in real-time |
| `--json` | off | JSON output |

### workflow metrics

```bash
ailoop workflow metrics --workflow deploy-pipeline --json
```

### workflow validate

```bash
ailoop workflow validate path/to/workflow.yaml
```

### workflow history

```bash
ailoop workflow history
```

## config -- Interactive configuration

```bash
ailoop config --init
ailoop config --init --config-file /path/to/config.toml
```

Default config location: `~/.config/ailoop/config.toml`

## provider -- Provider management

```bash
ailoop provider list              # List configured providers
ailoop provider telegram test     # Test Telegram integration
```

## Channel Rules

Channels isolate workflows. Names: 1-64 chars, lowercase alphanumeric, hyphens, underscores; must start with letter or digit. Default channel is `public`.

## Telegram Provider Setup

1. Create bot via [@BotFather](https://t.me/BotFather), copy token.
2. Start a chat with your bot (search username, open chat, tap Start or send `/start`).
3. Get chat ID (e.g. [@userinfobot](https://t.me/userinfobot) or group ID).
4. Set token: `export AILOOP_TELEGRAM_BOT_TOKEN=your_bot_token`
5. Configure: `ailoop config --init` and enable Telegram with `chat_id`.
6. Test: `ailoop provider telegram test`
7. Run server: `ailoop serve` -- questions/authorizations answered in Telegram or terminal.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection refused on forward | Ensure `ailoop serve` is running (default WS port 8080). Use `--url ws://HOST:PORT`. |
| No messages visible on server | Run `ailoop serve` in an external terminal (real TTY), not IDE integrated terminal. |
| Testing WebSocket delivery | Use [websocat](https://github.com/vi/websocat) to replay captured messages. |
