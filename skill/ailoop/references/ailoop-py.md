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
async with AiloopClient(server_url="http://localhost:8080") as client:
    msg = await client.ask("Ready to proceed?")
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

### ask -- Ask a question

```python
msg = await client.ask(
    question="What approach?",
    channel="dev-review",
    timeout=60,
    choices=["Option A", "Option B", "Option C"],
)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `question` | `str` | required | Question text |
| `channel` | `str \| None` | client default | Target channel |
| `timeout` | `int \| None` | `60` | Response timeout in seconds |
| `choices` | `list[str] \| None` | `None` | Multiple choice options |

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

### Connect and register handlers

```python
from ailoop import AiloopClient

client = AiloopClient(server_url="http://localhost:8080")

async def on_message(data: dict):
    content = data.get("content", {})
    msg_type = content.get("type")

    if msg_type == "question":
        print(f"Question: {content['text']}")
    elif msg_type == "authorization":
        print(f"Auth request: {content['action']}")
    elif msg_type == "notification":
        print(f"Notification: {content['text']}")
    elif msg_type == "response":
        print(f"Response: {content.get('answer')}")
    else:
        print(f"Message: {data}")

async def on_connection(event: dict):
    print(f"Connection event: {event['type']}")

client.add_message_handler(on_message)
client.add_connection_handler(on_connection)

await client.connect()
await client.connect_websocket()
await client.subscribe_to_channel("public")

try:
    await asyncio.Future()  # run forever
finally:
    await client.disconnect_websocket()
    await client.disconnect()
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

### Important limitations

- `ask()` and `authorize()` return the **sent** message, not the human's reply. To receive responses, register a handler via `add_message_handler()` and match on `correlation_id`.
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
Message.create_question(channel, text, timeout_seconds=60, choices=None)
Message.create_authorization(channel, action, timeout_seconds=300, context=None)
Message.create_notification(channel, text, priority=NotificationPriority.NORMAL)
Message.create_response(channel, correlation_id, answer=None, response_type=ResponseType.TEXT)
```

Fields: `id` (UUID), `channel`, `sender_type`, `content`, `timestamp`, `correlation_id`, `metadata`.

### Content types (discriminated union on `type` field)

| Type | Content class | Key fields |
|------|---------------|------------|
| `question` | `QuestionContent` | `text`, `timeout_seconds`, `choices` |
| `authorization` | `AuthorizationContent` | `action`, `timeout_seconds`, `context` |
| `notification` | `NotificationContent` | `text`, `priority` |
| `response` | `ResponseContent` | `answer`, `response_type` |
| `navigate` | `NavigateContent` | `url` |

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
