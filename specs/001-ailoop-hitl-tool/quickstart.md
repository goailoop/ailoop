# ailoop Quick Start Guide

## Overview
ailoop is a human-in-the-loop CLI tool that enables AI agents to communicate with human users through structured interactions. This guide will get you up and running in minutes.

## Prerequisites

- **Operating System**: Linux or Windows (macOS support coming later)
- **Terminal**: Any modern terminal with UTF-8 support
- **Network**: Local network access (for server mode)

## Installation

```bash
# Download the latest release for your platform
curl -L https://github.com/your-org/ailoop/releases/latest/download/ailoop-linux-x64.tar.gz | tar xz
# or for Windows:
# curl -L https://github.com/your-org/ailoop/releases/latest/download/ailoop-windows-x64.zip -o ailoop.zip && unzip ailoop.zip

# Make executable (Linux/Mac)
chmod +x ailoop

# Test installation
./ailoop --version
```

## Basic Usage

### 1. Ask a Question (Direct Mode)

```bash
# Ask a simple question
./ailoop ask "What is the best approach for this task?"

# With timeout
./ailoop ask "Should we proceed?" --timeout 60

# Specify channel
./ailoop ask "Review this code change" --channel dev-review
```

### 2. Request Authorization

```bash
# Request approval for an action
./ailoop authorize "Deploy version 1.2.3 to production"

# With custom timeout and channel
./ailoop authorize "Delete user data" --timeout 300 --channel admin-ops
```

### 3. Send Notifications

```bash
# Send a notification
./ailoop say "Build completed successfully"

# With priority
./ailoop say "System alert: High CPU usage detected" --priority high
```

## Server Mode

For multi-agent environments, run ailoop in server mode:

```bash
# Start server on default port (8080)
./ailoop serve

# Start on custom port
./ailoop serve --port 9000

# Start with custom host
./ailoop serve --host 0.0.0.0 --port 8080
```

### Connecting to Server

```bash
# Use server mode for commands
./ailoop ask "Server test question" --server http://localhost:8080

# All commands support server mode
./ailoop authorize "Server authorization" --server http://localhost:8080
./ailoop say "Server notification" --server http://localhost:8080
```

## Configuration

### Interactive Setup

```bash
# Run interactive configuration
./ailoop config --init

# This will prompt for:
# - Default timeout settings
# - Default channel name
# - Server host/port preferences
# - Log level preferences
```

### Manual Configuration

Create `~/.config/ailoop/config.toml`:

```toml
# Default timeout in seconds (0 = no timeout)
timeout_seconds = 300

# Default channel for messages
default_channel = "public"

# Logging level
log_level = "info"

# Server settings
server_host = "127.0.0.1"
server_port = 8080

# Connection limits
max_connections = 100
```

## AI Agent Integration

### Python Example

```python
import subprocess
import json

def ask_human(question, channel="public", timeout=60):
    """Ask a human user a question via ailoop"""
    cmd = ["ailoop", "ask", question, "--channel", channel, "--timeout", str(timeout), "--json"]

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode == 0:
        response = json.loads(result.stdout)
        return response["response"]
    else:
        raise Exception(f"ailoop error: {result.stderr}")

def request_authorization(action, channel="admin"):
    """Request authorization for an action"""
    cmd = ["ailoop", "authorize", action, "--channel", channel, "--json"]

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode == 0:
        auth = json.loads(result.stdout)
        return auth["authorized"]
    elif result.returncode == 1:
        return False  # Denied
    else:
        raise Exception(f"Authorization error: {result.stderr}")

# Usage
response = ask_human("Should we proceed with the deployment?")
authorized = request_authorization("Delete production database")
```

### JavaScript/Node.js Example

```javascript
const { spawn } = require('child_process');

function askHuman(question, options = {}) {
  return new Promise((resolve, reject) => {
    const args = ['ask', question];

    if (options.channel) args.push('--channel', options.channel);
    if (options.timeout) args.push('--timeout', options.timeout.toString());
    args.push('--json');

    const ailoop = spawn('ailoop', args, { stdio: 'pipe' });

    let stdout = '';
    let stderr = '';

    ailoop.stdout.on('data', (data) => stdout += data.toString());
    ailoop.stderr.on('data', (data) => stderr += data.toString());

    ailoop.on('close', (code) => {
      if (code === 0) {
        resolve(JSON.parse(stdout));
      } else {
        reject(new Error(`ailoop error ${code}: ${stderr}`));
      }
    });
  });
}

// Usage
askHuman("What is your preferred approach?")
  .then(response => console.log('Human response:', response.response))
  .catch(error => console.error('Error:', error.message));
```

## Channel Management

### Channel Naming Rules

- **Format**: `^[a-z0-9][a-z0-9_-]{0,63}$`
- **Starts with**: Lowercase letter or number
- **Characters**: Letters, numbers, hyphens, underscores only
- **Length**: 1-64 characters

### Examples

```bash
# Valid channel names
ailoop ask "Question" --channel dev-team
ailoop ask "Question" --channel project-alpha
ailoop ask "Question" --channel user_123

# Default channel (if not specified)
ailoop ask "Question"  # Uses 'public' channel
```

## Troubleshooting

### Common Issues

**Command not found**
```bash
# Ensure ailoop is in your PATH
export PATH=$PATH:/path/to/ailoop
# Or run with full path
/path/to/ailoop --version
```

**Server won't start**
```bash
# Check if port is in use
netstat -tlnp | grep :8080
# Try different port
ailoop serve --port 9000
```

**Permission denied**
```bash
# For ports < 1024, may need sudo (not recommended for production)
# Use ports >= 1024
ailoop serve --port 8080
```

**Timeout errors**
```bash
# Increase timeout or set to 0 for no timeout
ailoop ask "Question" --timeout 300
ailoop ask "Question" --timeout 0  # No timeout
```

### Debug Mode

```bash
# Enable verbose logging
ailoop ask "Debug question" --verbose

# Check server logs
ailoop serve --verbose
```

## Next Steps

- **Documentation**: Read the full specification in `spec.md`
- **Integration**: Study the API contracts in `contracts/`
- **Development**: Review the data models in `data-model.md`
- **Planning**: Check the implementation plan in `plan.md`

For production deployments, consider:
- Setting up proper logging and monitoring
- Configuring firewalls for server access
- Implementing backup strategies for configuration files
- Planning for scalability as usage grows