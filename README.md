# ailoop

Human-in-the-loop CLI and SDK stack for AI agents. Agents can ask questions, request approvals, push notifications, and stream output through channels so people can supervise critical steps against a single ailoop server.

## Developer guide

Developer architecture, system design, compile, and test instructions are documented in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Install the CLI

- Releases: [GitHub Releases](https://github.com/goailoop/ailoop/releases)
- Homebrew (Linux): `brew install goailoop/cli/ailoop`
- Scoop (Windows):

```powershell
scoop bucket add goailoop https://github.com/goailoop/scoop
scoop install ailoop
```

Verify:

```bash
ailoop --version
```

## Quick start

### Run the server

```bash
ailoop serve
```

By default one process listens on `http://127.0.0.1:8080` (REST + WebSocket on the same port). Optional embedded monitor UI:

```bash
ailoop serve --web
```

Then open `http://127.0.0.1:8080` in a browser (see `examples/web-ui/README.md`).

### Single-port migration (v0.1.x → v0.1.40+)

Port **8081** is no longer used. Point health checks, firewalls, and clients at **8080** (or whatever you pass to `--port`).

### Set the server URL

```bash
export AILOOP_SERVER=http://127.0.0.1:8080
```

Override per command with `--server` (or `--url` on `forward`).

### Typical CLI usage

```bash
ailoop ask --payload '{"decision_id":"deploy","summary":"Deploy now?","options":[{"id":"yes","label":"Yes"},{"id":"no","label":"No"}]}'
ailoop authorize "Deploy version 1.2.3?" --default no
ailoop say "Build finished" --priority normal
ailoop navigate "https://example.com/review"
ailoop forward --channel public --agent-type cursor
```

Use `ailoop <command> --help` for flags and formats.

### CLI commands (summary)

| Command | Role |
|---------|------|
| `ask` | Structured decision; waits for human answer (use `--payload`; `--decision-json` is accepted as a deprecated alias) |
| `authorize` | Approval; timeouts and interruptions resolve to deny |
| `say` | Notification with priority |
| `navigate` | Confirm opening a URL |
| `image` | Show image (path or URL) to the human |
| `serve` | Run the ailoop server |
| `forward` | Stream agent output to the server (stdin, pipe, or `--input`) |
| `config` | Interactive config (`--init`) |
| `provider` | Provider status / Telegram test |
| `task` | Task storage subcommands |

## Workflow engine removed

As of v1.0.0, the embedded YAML/bash workflow engine (`ailoop workflow`) has been removed. Use external orchestrators (Newton, GitHub Actions, shell scripts) instead. See [`CHANGELOG.md`](CHANGELOG.md) for the full migration guide, including how to clean up `~/.ailoop/workflow_store.json`.

## SDKs

### TypeScript (`ailoop-js`)

```bash
npm install ailoop-js
```

```typescript
import { AiloopClient } from "ailoop-js";

const client = new AiloopClient({ baseURL: "http://127.0.0.1:8080" });
await client.say("public", "hello from js", "normal");
```

Details: [`ailoop-js/README.md`](ailoop-js/README.md).

### Python (`ailoop-py`)

```bash
pip install ailoop-py
```

```python
import asyncio
from ailoop import AiloopClient

async def main() -> None:
    client = AiloopClient("http://127.0.0.1:8080", channel="public")
    await client.say("hello from python")
    await client.ask("approve rollout?", timeout=30)

asyncio.run(main())
```

Details: [`ailoop-py/README.md`](ailoop-py/README.md).

## Telegram provider

1. Create a bot with [@BotFather](https://t.me/BotFather) and copy the token.
2. Open a chat **with your bot** and send `/start`.
3. `export AILOOP_TELEGRAM_BOT_TOKEN=<token>`
4. `ailoop config --init` and enable Telegram with your numeric chat ID (see [@userinfobot](https://t.me/userinfobot) if needed).
5. `ailoop provider telegram test`, then run `ailoop serve` so the server can deliver and collect replies.

## Channels

Isolation key for workloads. Allowed names are 1–64 characters; start with a letter or digit; use lowercase letters, digits, `-`, and `_`. Default channel name is `public`.

## Troubleshooting

- **Connection refused:** start `ailoop serve` (or adjust `--server` / `forward --url`).
- **No live server logs in an IDE terminal:** stdout may be fully buffered without a real TTY; run `ailoop serve` in an external terminal if you need streaming logs.

## More documentation

- Design: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Kubernetes: [`k8s/README.md`](k8s/README.md)
- Docker: [`README-Docker.md`](README-Docker.md)
- Web UI example: [`examples/web-ui/README.md`](examples/web-ui/README.md)

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Dual-licensed under **MIT OR Apache-2.0** (see crate and package metadata). Full Apache 2.0 text: [`docs/LICENSE`](docs/LICENSE).
