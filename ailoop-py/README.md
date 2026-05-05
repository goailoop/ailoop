# ailoop-py

Async Python client for an [ailoop](https://github.com/goailoop/ailoop) server (`ask`, `authorize`, `say`, `navigate`, WebSocket subscriptions, and related APIs).

## Requirements

- Python 3.11+

## Install

```bash
pip install ailoop-py
```

PyPI distribution name is `ailoop-py`; imports use the `ailoop` package.

## Quick start

```python
import asyncio
from ailoop import AiloopClient

async def main() -> None:
    client = AiloopClient("http://127.0.0.1:8080", channel="public")
    await client.say("hello from python")
    await client.ask("approve rollout?", timeout=30)

asyncio.run(main())
```

## Capabilities

- Send `ask`, `authorize`, `say`, and `navigate`
- Fetch messages by ID and send responses
- Subscribe to channels over WebSocket with handlers

Use the package API docs or source under `src/ailoop/` for the full surface.

## Contributing

Developing this package in the monorepo (venv, `pip install -e`, tests, lint): [CONTRIBUTING.md](../CONTRIBUTING.md).
