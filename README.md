# ailoop

ailoop is a human-in-the-loop CLI and server for AI agents. Agents can ask structured questions, request approval, send notifications, display images, and stream output to people through a shared channel.

## Install

Download a binary from [GitHub Releases](https://github.com/goailoop/ailoop/releases), or use a package manager.

Homebrew on Linux:

```bash
brew install goailoop/cli/ailoop
```

Scoop on Windows:

```powershell
scoop bucket add goailoop https://github.com/goailoop/scoop-bucket
scoop install ailoop
```

Verify the installation:

```bash
ailoop --version
```

## Quick start

Start the server with its embedded monitor UI:

```bash
ailoop serve --web
```

Open `http://127.0.0.1:8080`, then use another terminal to send an interaction:

```bash
ailoop say "Build finished" --priority normal
ailoop authorize "Deploy version 1.2.3?" --default no
```

The REST API, WebSocket endpoint, and monitor UI use the same port. The default server URL is `http://127.0.0.1:8080`.

## Structured interactions

Ask a question with explicit options:

```bash
ailoop ask --payload '{"decision_id":"deploy","summary":"Deploy now?","options":[{"id":"yes","label":"Yes"},{"id":"no","label":"No"}]}'
```

Other common interactions:

```bash
ailoop navigate "https://example.com/review"
ailoop image ./report.png
ailoop forward --channel public --agent-type cursor
```

`authorize` resolves timeouts and interruptions as denial. Use `ailoop <command> --help` for each command's flags and input formats.

## Commands

| Command | Purpose |
|---|---|
| `ask` | Send a structured decision and wait for an answer |
| `authorize` | Request approval with deny-by-default behavior |
| `say` | Send a notification with a priority |
| `navigate` | Ask the human to open a URL |
| `image` | Show a local image or image URL |
| `forward` | Stream agent output from stdin, a pipe, or `--input` |
| `serve` | Run the REST and WebSocket server |
| `config` | Create or update local configuration |
| `provider` | Inspect providers and test Telegram |
| `task` | Create, inspect, and update stored tasks |
| `doctor` | Check local configuration and server connectivity |

## Server and channels

Set a different default server URL with `AILOOP_SERVER`, or use `--server` for one command (`forward` uses `--url`).

Channels isolate workloads on one server. Channel names are 1–64 characters, start with a letter or digit, and use lowercase letters, digits, `-`, or `_`. The default channel is `public`.

## Telegram

1. Create a bot with [@BotFather](https://t.me/BotFather) and copy its token.
2. Open a chat with the bot and send `/start`.
3. Set `AILOOP_TELEGRAM_BOT_TOKEN` to the bot token.
4. Run `ailoop config --init`, enable Telegram, and enter the numeric chat ID.
5. Run `ailoop provider telegram test`.
6. Keep `ailoop serve` running to deliver interactions and collect replies.

## Client libraries

This repository contains source clients for [TypeScript](ailoop-js/README.md) and [Python](ailoop-py/README.md). Their READMEs document the source package APIs and development setup.

## Migration notes

- Since v0.1.40, port `8081` is no longer used. Point clients, health checks, and firewalls at port `8080` or the value passed to `--port`.
- Since v1.0.0, the embedded `ailoop workflow` YAML and shell workflow engine has been removed. Use an external orchestrator such as GitHub Actions, Newton, or shell scripts. See [CHANGELOG.md](CHANGELOG.md) for cleanup instructions.

## More documentation

- [Architecture](ARCHITECTURE.md)
- [Docker](README-Docker.md)
- [Kubernetes](k8s/README.md)
- [Web UI example](examples/web-ui/README.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for architecture, development, and test instructions.

## License

Dual-licensed under MIT or Apache-2.0. See the crate and package metadata and the [Apache-2.0 license text](docs/LICENSE).
