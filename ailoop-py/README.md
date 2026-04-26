# ailoop-py

Python SDK for integrating applications with an ailoop server.

## Requirements

- Python 3.11+

## Install

```bash
pip install ailoop-py
```

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

## Main capabilities

- Send `ask`, `authorize`, `say`, and `navigate` messages
- Retrieve existing messages by ID
- Send response payloads
- Subscribe to channels over WebSocket and attach handlers

## Local development

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
ruff check .
black --check .
pytest
```

## Notes

- SDK package name: `ailoop-py`
- Import root: `from ailoop import AiloopClient`

## Contributing

Use root workflow in `../CONTRIBUTING.md`.
