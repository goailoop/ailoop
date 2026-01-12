# ailoop

Human-in-the-Loop CLI Tool for AI Agent Communication

ailoop is a command-line tool that enables AI agents to communicate with human users through structured interactions. It supports both direct mode (for single-agent scenarios) and server mode (for multi-agent environments).

## Installation

### Linux

```bash
# Download the latest release
curl -L https://github.com/your-org/ailoop/releases/latest/download/ailoop-linux-x64.tar.gz | tar xz

# Make executable
chmod +x ailoop

# Move to PATH (optional)
sudo mv ailoop /usr/local/bin/
```

### Windows

```bash
# Download the latest release
curl -L https://github.com/your-org/ailoop/releases/latest/download/ailoop-windows-x64.zip -o ailoop.zip
unzip ailoop.zip

# Add to PATH (optional)
```

## Quick Start

### Ask a Question

Ask a question and wait for a human response:

```bash
# Simple question
ailoop ask "What is the best approach for this task?"

# With timeout (60 seconds)
ailoop ask "Should we proceed?" --timeout 60

# Specify a channel
ailoop ask "Review this code change" --channel dev-review

# JSON output
ailoop ask "What is your name?" --json
```

### Request Authorization

Request human approval for critical actions:

```bash
# Request authorization
ailoop authorize "Deploy version 1.2.3 to production"

# With custom timeout and channel
ailoop authorize "Delete user data" --timeout 300 --channel admin-ops

# JSON output
ailoop authorize "Execute dangerous operation" --json
```

**Note:** If no response is received within the timeout period, authorization defaults to **DENIED** for security.

### Send Notifications

Send notification messages to human users:

```bash
# Simple notification
ailoop say "Build completed successfully"

# With priority level
ailoop say "System alert: High CPU usage detected" --priority high

# Available priorities: low, normal, high, urgent
ailoop say "Critical error occurred" --priority urgent --channel alerts
```

### Display Images

Show images to users (file path or URL):

```bash
# Display image from file
ailoop image /path/to/image.png --channel review

# Display image from URL
ailoop image https://example.com/chart.png --channel analytics
```

### Navigate to URL

Suggest users navigate to a specific URL:

```bash
# Suggest navigation
ailoop navigate https://example.com/dashboard --channel monitoring
```

The tool will attempt to open the URL in your default browser automatically.

## Server Mode

For multi-agent environments, run ailoop in server mode:

```bash
# Start server on default port (8080)
ailoop serve

# Start on custom port
ailoop serve --port 9000

# Start with custom host and channel
ailoop serve --host 0.0.0.0 --port 8080 --channel default
```

The server provides an interactive terminal UI showing:
- Server status and metrics
- Queue size and active connections
- Recent activity

Press `Ctrl+C` to stop the server.

### Connecting to Server

When a server is running, you can use server mode for commands:

```bash
# Use server mode for commands
ailoop ask "Server test question" --server http://localhost:8080
ailoop authorize "Server authorization" --server http://localhost:8080
ailoop say "Server notification" --server http://localhost:8080
```

## Configuration

Set up your configuration interactively:

```bash
# Initialize configuration
ailoop config --init

# Use custom config file location
ailoop config --init --config-file ~/.config/ailoop/custom.toml
```

The configuration wizard will prompt you for:
- Default timeout for questions
- Default channel name
- Log level (error/warn/info/debug/trace)
- Server bind address and port

Configuration is saved in TOML format at `~/.config/ailoop/config.toml` by default.

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

## Command Options

### Common Options

Most commands support these options:

- `--channel, -c`: Specify channel name (default: `public`)
- `--server`: Server URL for remote operation (default: `http://127.0.0.1:8080`)
- `--json`: Output results in JSON format

### Timeout Behavior

- Timeout of `0` means wait indefinitely
- For questions: timeout results in exit code 1
- For authorizations: timeout defaults to **DENIED** for security
- Press `Ctrl+C` to cancel at any time

### Exit Codes

- `0`: Success
- `1`: Error, timeout, or denied authorization
- `130`: Cancelled by user (Ctrl+C)

## Examples

### Example 1: Code Review Workflow

```bash
# Agent asks for code review
ailoop ask "Please review PR #123" --channel code-review --timeout 3600

# Agent requests approval
ailoop authorize "Merge PR #123 to main branch" --channel code-review
```

### Example 2: Deployment Workflow

```bash
# Start server for deployment team
ailoop serve --port 8080 --channel deployments

# In another terminal, request deployment approval
ailoop authorize "Deploy v2.1.0 to production" \
  --server http://localhost:8080 \
  --channel deployments \
  --timeout 600
```

### Example 3: Monitoring and Alerts

```bash
# Send urgent alert
ailoop say "Database connection pool exhausted!" \
  --priority urgent \
  --channel monitoring

# Show monitoring dashboard
ailoop navigate https://monitoring.example.com/dashboard \
  --channel monitoring
```

## Troubleshooting

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

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) file for details.
