# ailoop-py Reference

Python SDK for ailoop server communication. Client-only -- requires a running `ailoop serve` instance.

## Installation

```bash
pip install ailoop-py
```

Dependencies: `httpx>=0.24`, `websockets>=11`, `pydantic>=2`, `typing-extensions>=4.5`. Requires Python >= 3.11.

## Setup

```python
from ailoop import AiloopClient

client = AiloopClient(
    server_url="http://localhost:8080",
    channel="public",
    timeout=30.0,
    reconnect_attempts=5,
    reconnect_delay=1.0,
)
```

### Async context manager (recommended)

Manages HTTP client lifecycle and WebSocket connection automatically:

```python
from ailoop.models import DecisionOption
async with AiloopClient(server_url="http://localhost:8080") as client:
    msg = await client.ask_decision(
        decision_id="proceed-check",
        summary="Ready to proceed?",
        options=[DecisionOption(id="yes", label="Yes"), DecisionOption(id="no", label="No")],
    )
    await client.say("Done")
```

### Manual lifecycle

```python
client = AiloopClient(server_url="http://localhost:8080")
await client.connect()           # HTTP client + health check
await client.connect_websocket() # WebSocket loop in background
# ... use client ...
await client.disconnect_websocket()
await client.disconnect()
```

## Sending Messages

All send methods POST to `/api/v1/messages` and return the server-created `Message`.

### ask_decision -- Send a structured decision

```python
from ailoop.models import DecisionOption, DecisionRecommendation

msg = await client.ask_decision(
    decision_id="deploy-strategy",
    summary="Which deployment strategy?",
    options=[
        DecisionOption(id="blue-green", label="Blue/Green", detail_markdown="Zero-downtime swap."),
        DecisionOption(id="canary", label="Canary (10%)", detail_markdown="Gradual rollout."),
        DecisionOption(id="rollback", label="Rollback to v1.4.2"),
    ],
    channel="dev-review",
    timeout=300,
    context_markdown="Current error rate: **0.3%**.",
    recommendation=DecisionRecommendation(
        option_id="blue-green",
        rationale_markdown="Fastest recovery.",
    ),
)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `decision_id` | `str` | required | Stable agent-assigned identifier |
| `summary` | `str` | required | Short question/heading |
| `options` | `list[DecisionOption]` | required | ≥2 options with unique non-empty `id` |
| `channel` | `str \| None` | client default | Target channel |
| `timeout` | `int \| None` | `300` | Response timeout in seconds |
| `context_markdown` | `str \| None` | `None` | Optional markdown context block |
| `recommendation` | `DecisionRecommendation \| None` | `None` | Agent's preferred option |

The response `Message.content.answer` always contains the resolved canonical option `id`. Response `metadata` includes `option_id`, `label`, and `index`.

### authorize -- Request authorization

```python
msg = await client.authorize(
    action="Deploy v2.0 to production",
    channel="admin-ops",
    timeout=300,
    context={"version": "2.0", "environment": "prod"},
)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `action` | `str` | required | Action description |
| `channel` | `str \| None` | client default | Target channel |
| `timeout` | `int \| None` | `300` | Timeout in seconds (denied on expiry) |
| `context` | `dict \| None` | `None` | Additional metadata |

### say -- Send a notification

```python
msg = await client.say(
    message="Build completed",
    channel="monitoring",
    priority="high",
)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `message` | `str` | required | Notification text |
| `channel` | `str \| None` | client default | Target channel |
| `priority` | `NotificationPriority` | `NORMAL` | `LOW`, `NORMAL`, `HIGH`, `URGENT` |

### navigate -- Send a navigation URL

```python
msg = await client.navigate(
    url="https://dashboard.example.com/deploy/123",
    channel="public",
)
```

### respond -- Reply to a message

Fetches the original message to determine its channel, then sends a response with `correlation_id` set.

```python
response = await client.respond(
    original_message_id="550e8400-e29b-41d4-a716-446655440000",
    answer="Yes, proceed",
    response_type="text",
)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `original_message_id` | `str \| UUID` | required | ID of the message to respond to |
| `answer` | `str \| None` | `None` | Response text |
| `response_type` | `ResponseType` | `TEXT` | `TEXT`, `AUTHORIZATION_APPROVED`, `AUTHORIZATION_DENIED`, `TIMEOUT`, `CANCELLED` |

### get_message -- Retrieve a message

```python
msg = await client.get_message("550e8400-e29b-41d4-a716-446655440000")
```

## Listening for Messages (WebSocket)

The SDK provides real-time message reception via WebSocket with handler callbacks.

### Handler registration order

**`add_message_handler()` and `add_connection_handler()` MUST be called before entering the `async with` block** (or before calling `connect_websocket()` manually).

`__aenter__` immediately calls `connect_websocket()`, which spawns a background `asyncio.Task` (`_websocket_loop`) that begins dispatching inbound messages and connection events. Any handler registered after `__aenter__` starts can silently miss messages — including the initial `{"type": "connected"}` connection event — that arrived between the task spawn and the registration call.

Both methods are synchronous list-appends and are safe to call at any point before the context manager is entered:

```python
client = AiloopClient("http://localhost:8080")
client.add_message_handler(on_message)       # MUST precede async with
client.add_connection_handler(on_connection) # MUST precede async with

async with client:
    ...
```

### What `async with` does

**`__aenter__`** (in order):
1. `connect()` — initialises `httpx.AsyncClient`, calls `check_version_compatibility()`. Raises `ConnectionError` if the server is unreachable.
2. `connect_websocket()` — spawns `_websocket_loop` as a background `asyncio.Task` that opens the WebSocket and starts dispatching messages to registered handlers.

**`__aexit__`** (always, even on exception):
1. `disconnect_websocket()` — cancels the background task and awaits its completion, closes the WebSocket.
2. `disconnect()` — closes the `httpx.AsyncClient` (also tears down the WebSocket task if somehow still running).

### Manual lifecycle

Use manual lifecycle when you cannot use the context manager (e.g., class-level client):

```python
client = AiloopClient(server_url="http://localhost:8080")
client.add_message_handler(on_message)

await client.connect()            # 1. HTTP client init + version check
await client.connect_websocket()  # 2. Spawns background receive loop
await client.subscribe_to_channel("public")  # 3. Subscribe (requires ws open)

# ... do work ...

await client.disconnect_websocket()  # 4. Cancel background task
await client.disconnect()            # 5. Close HTTP client
```

Calling `disconnect()` alone also tears down the WebSocket task if still running.

### Correlation ID and reply matching

`ask()` and `authorize()` POST to `/api/v1/messages` and return the **sent** `Message` object. They do **not** await the human reply.

To correlate a human `response` event received via WebSocket to a specific `ask`/`authorize` call, match `data["correlation_id"]` against `str(sent_message.id)`. Using a `dict[str, asyncio.Future]` keyed by correlation ID supports multiple concurrent requests:

```python
pending: dict[str, asyncio.Future] = {}

async def on_message(data: dict) -> None:
    cid = data.get("correlation_id")
    if cid and cid in pending:
        pending[cid].set_result(data)

# Before async with:
client.add_message_handler(on_message)

async with client:
    await client.subscribe_to_channel("public")
    from ailoop.models import DecisionOption
    sent = await client.ask_decision(
        decision_id="proceed-check",
        summary="Proceed?",
        options=[DecisionOption(id="yes", label="Yes"), DecisionOption(id="no", label="No")],
        timeout=60,
    )
    fut: asyncio.Future = asyncio.get_event_loop().create_future()
    pending[str(sent.id)] = fut
    reply = await fut
    print(reply["content"].get("answer"))
```

### Complete runnable example

See [`ailoop-py/examples/streaming_agent.py`](../../../../ailoop-py/examples/streaming_agent.py) for a full example covering handlers, `async with`, channel subscription, `ask`, correlated reply, and clean `asyncio.Event`-based shutdown.

```python
from ailoop import AiloopClient
import asyncio

async def main() -> None:
    pending: dict[str, asyncio.Future] = {}
    stop = asyncio.Event()

    async def on_message(data: dict) -> None:
        content = data.get("content", {})
        cid = data.get("correlation_id")
        if content.get("type") == "response" and cid and cid in pending:
            pending[cid].set_result(data)
            stop.set()
        else:
            print(f"[message] type={content.get('type')} channel={data.get('channel')}")

    async def on_connection(event: dict) -> None:
        print(f"[connection] {event['type']}")

    client = AiloopClient("http://127.0.0.1:8080", channel="public")
    client.add_message_handler(on_message)       # MUST be before async with
    client.add_connection_handler(on_connection)

    async with client:
        await client.subscribe_to_channel("public")
        from ailoop.models import DecisionOption
        sent = await client.ask_decision(
            decision_id="deployment-check",
            summary="Proceed with deployment?",
            options=[DecisionOption(id="yes", label="Yes"), DecisionOption(id="no", label="No")],
            timeout=120,
        )
        fut: asyncio.Future = asyncio.get_event_loop().create_future()
        pending[str(sent.id)] = fut
        await stop.wait()
        reply = fut.result()
        print(f"[reply] {reply['content'].get('answer')}")

if __name__ == "__main__":
    asyncio.run(main())
```

### Channel subscriptions

```python
await client.subscribe_to_channel("dev-review")
await client.unsubscribe_from_channel("dev-review")
```

Subscriptions are automatically re-established on reconnection.

### Reconnection

The WebSocket loop reconnects automatically with exponential backoff. Configured by:
- `reconnect_attempts` (default 5): max reconnection tries
- `reconnect_delay` (default 1.0s): base delay, doubled each attempt

After `reconnect_attempts` consecutive failures the background task exits silently (no exception is raised). Monitor connection health by checking `{"type": "connected"}` events in an `add_connection_handler()` callback — the absence of a reconnect event is the signal that the loop has stopped.

### Important limitations

- `ask()` and `authorize()` return the **sent** message, not the human's reply. To receive responses, register a handler via `add_message_handler()` and match on `correlation_id`. See [Correlation ID and reply matching](#correlation-id-and-reply-matching) above.
- Handlers receive raw JSON `dict` objects. There is no typed event dispatch or filtering built in.
- There is no `remove_message_handler()` -- create a new client to clear handlers.

## Task Management

### Create a task

```python
task = await client.create_task(
    title="Deploy service",
    description="Deploy v2 to staging",
    channel="ops",
    assignee="alice",
    metadata={"priority": "high"},
)
```

### Update task state

```python
task = await client.update_task(task_id="abc-123", state="done")
```

Valid states: `pending`, `done`, `abandoned`.

### List / get tasks

```python
tasks = await client.list_tasks(channel="ops", state="pending")
task = await client.get_task(task_id="abc-123")
```

### Dependencies

```python
await client.add_dependency(task_id="child-id", depends_on="parent-id", type="blocks")
await client.remove_dependency(task_id="child-id", depends_on="parent-id")
```

Dependency types: `blocks`, `related`, `parent`.

### Query dependency state

```python
ready = await client.get_ready_tasks(channel="ops")
blocked = await client.get_blocked_tasks(channel="ops")
graph = await client.get_dependency_graph(task_id="abc-123")
# graph = {"task": ..., "parents": [...], "children": [...]}
```

## Health & Version

```python
info = await client.check_version_compatibility()
# {"server_version": "0.1.7", "client_version": "0.1.1", "compatible": true/false}
```

## Models

### Message

Core envelope. Created via factory methods or received from server.

```python
Message.create_decision(channel, decision_id, summary, options, timeout_seconds=300, context_markdown=None, recommendation=None)
Message.create_authorization(channel, action, timeout_seconds=300, context=None)
Message.create_notification(channel, text, priority=NotificationPriority.NORMAL)
Message.create_response(channel, correlation_id, answer=None, response_type=ResponseType.TEXT)
```

Fields: `id` (UUID), `channel`, `sender_type`, `content`, `timestamp`, `correlation_id`, `metadata`.

### Content types (discriminated union on `type` field)

| Type | Content class | Key fields |
|------|---------------|------------|
| `decision` | `DecisionContent` | `decision_id`, `summary`, `options`, `context_markdown`, `recommendation`, `timeout_seconds` |
| `authorization` | `AuthorizationContent` | `action`, `timeout_seconds`, `context` |
| `notification` | `NotificationContent` | `text`, `priority` |
| `response` | `ResponseContent` | `answer`, `response_type` |
| `navigate` | `NavigateContent` | `url` |

### Decision models

```python
class DecisionOption(BaseModel):
    id: str                           # stable machine-readable id, unique within Decision
    label: str                        # human-readable label
    detail_markdown: Optional[str]    # optional markdown detail

class DecisionRecommendation(BaseModel):
    option_id: str                    # MUST match an options[].id
    rationale_markdown: Optional[str] # optional markdown rationale

class DecisionContent(BaseModel):
    type: Literal["decision"]
    decision_id: str
    summary: str
    context_markdown: Optional[str]
    options: List[DecisionOption]     # len >= 2, unique ids
    recommendation: Optional[DecisionRecommendation]
    timeout_seconds: int
```

### Enums

| Enum | Values |
|------|--------|
| `SenderType` | `AGENT`, `HUMAN` |
| `ResponseType` | `TEXT`, `AUTHORIZATION_APPROVED`, `AUTHORIZATION_DENIED`, `TIMEOUT`, `CANCELLED` |
| `NotificationPriority` | `LOW`, `NORMAL`, `HIGH`, `URGENT` |
| `TaskState` | `PENDING`, `DONE`, `ABANDONED` |
| `DependencyType` | `BLOCKS`, `RELATED`, `PARENT` |

## Exceptions

All inherit from `AiloopError`:

| Exception | When raised |
|-----------|-------------|
| `ConnectionError` | HTTP failures, WebSocket failures, server unreachable |
| `ValidationError` | Invalid input, 400 responses, 404 not-found |
| `TimeoutError` | Request timeout |

## REST API Endpoints

| Endpoint | Method | Client method(s) |
|----------|--------|-------------------|
| `/api/v1/messages` | POST | `ask`, `authorize`, `say`, `navigate` |
| `/api/v1/messages/{id}` | GET | `get_message`, `respond` |
| `/api/v1/tasks` | POST | `create_task` |
| `/api/v1/tasks` | GET | `list_tasks` |
| `/api/v1/tasks/{id}` | GET/PUT | `get_task`, `update_task` |
| `/api/v1/tasks/{id}/dependencies` | POST | `add_dependency` |
| `/api/v1/tasks/{id}/dependencies/{id}` | DELETE | `remove_dependency` |
| `/api/v1/tasks/ready` | GET | `get_ready_tasks` |
| `/api/v1/tasks/blocked` | GET | `get_blocked_tasks` |
| `/api/v1/tasks/{id}/graph` | GET | `get_dependency_graph` |
| `/api/v1/health` | GET | `check_version_compatibility` |
| `ws://{host}/ws` | WebSocket | `connect_websocket` |
