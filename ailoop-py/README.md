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

## WebSocket streaming

**Handlers must be registered before entering `async with AiloopClient(...)`.**
`__aenter__` immediately spawns the background receive loop — handlers registered after that point can miss messages silently.

See [`examples/streaming_agent.py`](examples/streaming_agent.py) for a complete runnable example covering handlers, channel subscription, `ask`, correlated reply, and clean shutdown.

Full SDK reference (connection lifecycle, correlation ID matching, manual lifecycle, workflow scope):
[`skill/ailoop/references/ailoop-py.md`](../skill/ailoop/references/ailoop-py.md) (monorepo-relative path).

## Capabilities

- Send structured decisions (`ask_decision`), `authorize`, `say`, and `navigate`
- Fetch messages by ID and send responses
- Subscribe to channels over WebSocket with handlers

Use the package API docs or source under `src/ailoop/` for the full surface.

## Migration: `create_question()` → `create_decision()`

`QuestionContent` and `Message.create_question()` have been removed. Use `DecisionContent`, `DecisionOption`, and `Message.create_decision()`:

```python
# Before
from ailoop.models import Message
msg = Message.create_question(channel="ops", text="Which strategy?", choices=["blue-green", "canary"])

# After
from ailoop.models import Message, DecisionOption, DecisionRecommendation
msg = Message.create_decision(
    channel="ops",
    decision_id="deploy-strategy",
    summary="Which deployment strategy?",
    options=[
        DecisionOption(id="blue-green", label="Blue/Green"),
        DecisionOption(id="canary", label="Canary (10%)"),
    ],
    recommendation=DecisionRecommendation(option_id="blue-green"),
)
```

The response `answer` is now always the canonical option `id` (not a raw label or number). Response metadata includes `option_id`, `label`, and `index`.

## Contributing

Developing this package in the monorepo (venv, `pip install -e`, tests, lint): [CONTRIBUTING.md](../CONTRIBUTING.md).
