# ailoop API Reference

HTTP REST API and WebSocket protocol exposed by `ailoop serve`. This is the transport layer that the CLI, Python SDK, and TypeScript SDK all use to communicate with the server.

## Server Endpoints

The server exposes two ports:

| Service | Default Address | Configurable |
|---------|----------------|--------------|
| WebSocket | `{host}:{port}` (default `127.0.0.1:8080`) | `ailoop serve --host --port` |
| HTTP REST API | `{host}:{port+1}` (default `127.0.0.1:8081`) | Derived from WS port |

No authentication or API key is required.

---

## HTTP REST API (v1)

Base path: `/api/v1`

### Health

#### `GET /api/v1/health`

Health check and version info.

**Response 200:**

```json
{
  "status": "healthy",
  "version": "0.1.7",
  "active_connections": 3,
  "queue_size": 0,
  "active_channels": 2
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | `string` | Always `"healthy"` |
| `version` | `string` | Server version |
| `active_connections` | `number` | Connected WebSocket clients |
| `queue_size` | `number` | Pending messages in queue |
| `active_channels` | `number` | Channels with messages |

---

### Messages

#### `POST /api/v1/messages`

Send a message. The message is stored in channel history and broadcast to all WebSocket subscribers on that channel.

**Request body:**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "channel": "my-channel",
  "sender_type": "AGENT",
  "content": { "type": "notification", "text": "Hello", "priority": "normal" },
  "timestamp": "2026-05-02T12:00:00Z",
  "correlation_id": null,
  "metadata": null
}
```

**Response 201:** The created `Message` (JSON).

**Channel validation:** 1-64 chars, starts with alphanumeric, only `[a-zA-Z0-9_-]`. Reserved names rejected: `system`, `admin`, `internal`, `reserved`, `ailoop`.

See [Message content types](#message-content-types) below for all valid `content` payloads.

---

#### `GET /api/v1/messages/:id`

Retrieve a message by its UUID.

**Response 200:** `Message` (JSON).

**Response 404:**

```json
{"error": "Message not found", "message_id": "..."}
```

---

#### `POST /api/v1/messages/:id/response`

Send a response to an existing message. If a terminal or Telegram prompt is waiting on the server, this completes it.

**Request body:**

```json
{
  "answer": "Yes, proceed",
  "response_type": "text"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `answer` | `string \| null` | Response text |
| `response_type` | `string` | See [Response types](#response-types) |

**Response 200:** The created response `Message` (with `correlation_id` set to the original message's `id`).

**Response 404:**

```json
{"error": "Original message not found", "message_id": "..."}
```

---

### Tasks

#### `POST /api/v1/tasks`

Create a new task.

**Request body:**

```json
{
  "title": "Deploy service",
  "description": "Deploy v2 to staging",
  "channel": "ops",
  "assignee": "alice",
  "metadata": {"priority": "high"}
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | `string` | yes | Task title |
| `description` | `string` | yes | Task description |
| `channel` | `string` | yes | Channel scope |
| `assignee` | `string \| null` | no | Assignee |
| `metadata` | `object \| null` | no | Custom metadata |

**Response 201:** `Task` (JSON). See [Task schema](#task-schema).

---

#### `GET /api/v1/tasks`

List tasks, optionally filtered by state.

**Query parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `channel` | `string` | yes | Channel scope |
| `state` | `string` | no | Filter: `pending`, `done`, `abandoned` |

**Response 200:**

```json
{
  "channel": "ops",
  "tasks": [...],
  "total_count": 5
}
```

Tasks are sorted by `created_at`.

---

#### `GET /api/v1/tasks/:id`

Get a task by UUID.

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `channel` | `string` | `public` | Channel scope |

**Response 200:** `Task` (JSON).

**Response 404:** `{"error": "Task not found", "task_id": "..."}`

---

#### `PUT /api/v1/tasks/:id`

Update a task's state. Triggers cascading blocked-status recalculation on dependent tasks.

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `channel` | `string` | `public` | Channel scope |

**Request body:**

```json
{"state": "done"}
```

Valid states: `pending`, `done`, `abandoned`.

**Response 200:** Updated `Task` (JSON).

**Response 404:** `{"error": "...", "task_id": "..."}`

---

#### `POST /api/v1/tasks/:id/dependencies`

Add a dependency between two tasks. Validates both tasks exist and prevents circular dependencies.

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `channel` | `string` | `public` | Channel scope |

**Request body:**

```json
{
  "child_id": "uuid-of-child",
  "parent_id": "uuid-of-parent",
  "dependency_type": "blocks"
}
```

Dependency types: `blocks`, `related`, `parent`.

**Response 200:** `{"status": "ok"}`

---

#### `DELETE /api/v1/tasks/:id/dependencies/:dep_id`

Remove a dependency.

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `channel` | `string` | `public` | Channel scope |

**Response 200:** `{"status": "ok"}`

---

#### `GET /api/v1/tasks/:id/dependencies`

Get a task's direct dependencies.

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `channel` | `string` | `public` | Channel scope |

**Response 200:**

```json
{
  "task_id": "...",
  "depends_on": ["<uuid>", ...],
  "blocking_for": ["<uuid>", ...]
}
```

**Response 404:** `{"error": "Task not found", "task_id": "..."}`

---

#### `GET /api/v1/tasks/:id/graph`

Get the full dependency graph for a task (all ancestors and descendants).

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `channel` | `string` | `public` | Channel scope |

**Response 200:**

```json
{
  "task": { ... },
  "parents": [...],
  "children": [...]
}
```

---

#### `GET /api/v1/tasks/ready`

Get tasks with no unresolved blockers.

**Query parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `channel` | `string` | yes | Channel scope |

**Response 200:** `TasksResponse` (same as list).

---

#### `GET /api/v1/tasks/blocked`

Get tasks that are blocked by unmet dependencies.

**Query parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `channel` | `string` | yes | Channel scope |

**Response 200:** `TasksResponse` (same as list).

---

## Legacy HTTP Routes (unversioned)

These routes are available but lack the `/v1/` prefix:

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/channels` | List all channels with stats |
| `GET` | `/api/channels/:channel/messages` | Message history for a channel |
| `GET` | `/api/channels/:channel/stats` | Channel statistics |
| `GET` | `/api/stats` | Broadcast statistics |

### `GET /api/channels`

**Response 200:**

```json
{
  "channels": [
    {
      "name": "public",
      "message_count": 42,
      "oldest_message": "2026-05-02T10:00:00Z",
      "newest_message": "2026-05-02T12:00:00Z"
    }
  ]
}
```

### `GET /api/channels/:channel/messages`

**Query parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `number` | `100` | Max messages to return |
| `offset` | `number` | `0` | (Defined but currently unused) |

**Response 200:**

```json
{
  "channel": "public",
  "messages": [...],
  "total_count": 42
}
```

Max 1000 messages stored per channel (FIFO eviction).

### `GET /api/channels/:channel/stats`

**Response 200:**

```json
{
  "channel": "public",
  "message_count": 42,
  "oldest_message": "2026-05-02T10:00:00Z",
  "newest_message": "2026-05-02T12:00:00Z"
}
```

### `GET /api/stats`

**Response 200:**

```json
{
  "total_viewers": 5,
  "agent_connections": 3,
  "viewer_connections": 2,
  "active_channels": 2
}
```

---

## WebSocket Protocol

### Connection

```
ws://{host}:{port}/
```

Default: `ws://127.0.0.1:8080/`

The server accepts WebSocket upgrades on the main port. No sub-protocol negotiation required.

### Channel subscription

Subscription is **implicit** -- when the server receives a message on a channel, it auto-subscribes the connection to that channel. Connections receive all subsequent messages broadcast on subscribed channels.

The SDK clients send explicit `subscribe`/`unsubscribe` JSON frames, but these are a client-side convention. The server-side subscription model is message-driven.

### Wire format

All frames are JSON text. Messages are serialized `Message` structs:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "channel": "my-channel",
  "sender_type": "AGENT",
  "content": { "type": "question", "text": "Ready?", "timeout_seconds": 60 },
  "timestamp": "2026-05-02T12:00:00Z",
  "correlation_id": null,
  "metadata": null
}
```

### Server processing flow

1. Client sends a `Message` as a WebSocket text frame
2. Server parses JSON into `Message` struct
3. Server auto-subscribes the connection to `message.channel`
4. Server stores message in per-channel history (max 1000, FIFO eviction)
5. Server broadcasts message to all other WebSocket subscribers on that channel
6. For interactive types (`question`, `authorization`, `navigate`):
   - Server sends to notification sinks (Telegram)
   - Server registers a pending prompt
   - Server races terminal input vs. external reply (via `POST /api/v1/messages/:id/response`)
7. Response message is broadcast to all channel subscribers
8. Original sender matches response by `correlation_id == original_message.id`

### Heartbeat

No explicit heartbeat or ping protocol. Connection liveness detected by close frames and send failures.

---

## Data Schemas

### Message

```
{
  "id":              UUID (string),
  "channel":         string,
  "sender_type":     "AGENT" | "HUMAN",
  "content":         MessageContent (discriminated by "type"),
  "timestamp":       RFC3339 datetime string,
  "correlation_id":  UUID | null,
  "metadata":        object | null
}
```

### Message content types

Discriminated union on the `"type"` field:

#### question

```json
{"type": "question", "text": "Ready?", "timeout_seconds": 60, "choices": ["A", "B"]}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | `string` | yes | Question text |
| `timeout_seconds` | `number` | yes | Response timeout |
| `choices` | `string[] \| null` | no | Multiple choice options |

#### authorization

```json
{"type": "authorization", "action": "Deploy v2", "timeout_seconds": 300, "context": {}}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `action` | `string` | yes | Action description |
| `timeout_seconds` | `number` | yes | Timeout (defaults to denied) |
| `context` | `object \| null` | no | Additional context |

#### notification

```json
{"type": "notification", "text": "Build done", "priority": "normal"}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | `string` | yes | Notification text |
| `priority` | `string` | yes | `low`, `normal`, `high`, `urgent` |

#### response

```json
{"type": "response", "answer": "Yes", "response_type": "text"}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `answer` | `string \| null` | no | Response text |
| `response_type` | `string` | yes | See below |

#### navigate

```json
{"type": "navigate", "url": "https://example.com"}
```

#### task_create

```json
{"type": "task_create", "task": { ... }}
```

#### task_update

```json
{"type": "task_update", "task_id": "uuid", "state": "done", "updated_at": "RFC3339"}
```

#### task_dependency_add

```json
{"type": "task_dependency_add", "task_id": "uuid", "depends_on": "uuid", "dependency_type": "blocks", "timestamp": "RFC3339"}
```

#### task_dependency_remove

```json
{"type": "task_dependency_remove", "task_id": "uuid", "depends_on": "uuid", "timestamp": "RFC3339"}
```

#### workflow_progress

```json
{"type": "workflow_progress", "execution_id": "uuid", "workflow_name": "deploy", "current_state": "build", "status": "running", "progress_percentage": 50}
```

#### workflow_completed

```json
{"type": "workflow_completed", "execution_id": "uuid", "workflow_name": "deploy", "final_status": "success", "duration_seconds": 120}
```

#### stdout / stderr

```json
{"type": "stdout", "execution_id": "uuid", "state_name": "build", "content": "...", "sequence": 1}
```

### Response types

| Value | Description |
|-------|-------------|
| `text` | Text answer to a question |
| `authorization_approved` | Authorization granted |
| `authorization_denied` | Authorization denied |
| `timeout` | Prompt timed out |
| `cancelled` | User cancelled |

### Task schema

```
{
  "id":              UUID (string),
  "title":           string,
  "description":     string,
  "state":           "pending" | "done" | "abandoned",
  "created_at":      RFC3339 datetime string,
  "updated_at":      RFC3339 datetime string,
  "assignee":        string | null,
  "metadata":        object | null,
  "depends_on":      string[],
  "blocking_for":    string[],
  "blocked":         boolean,
  "dependency_type": "blocks" | "related" | "parent" | null
}
```

### Error response format

```json
{"error": "Human-readable message", "message_id": "..."}
{"error": "Human-readable message", "task_id": "..."}
```

Validation errors return HTTP 400 with warp rejection handling. Not-found errors return HTTP 404.

---

## curl Examples

### Send a notification

```bash
curl -X POST http://localhost:8081/api/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "channel": "public",
    "sender_type": "AGENT",
    "content": {"type": "notification", "text": "Build done", "priority": "normal"},
    "timestamp": "2026-05-02T12:00:00Z"
  }'
```

### Ask a question

```bash
curl -X POST http://localhost:8081/api/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "channel": "public",
    "sender_type": "AGENT",
    "content": {"type": "question", "text": "Proceed?", "timeout_seconds": 60, "choices": ["yes", "no"]},
    "timestamp": "2026-05-02T12:00:00Z"
  }'
```

### Respond to a message

```bash
curl -X POST http://localhost:8081/api/v1/messages/660e8400-e29b-41d4-a716-446655440001/response \
  -H "Content-Type: application/json" \
  -d '{"answer": "yes", "response_type": "text"}'
```

### Check health

```bash
curl http://localhost:8081/api/v1/health
```

### Create a task

```bash
curl -X POST http://localhost:8081/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title": "Deploy", "description": "Deploy v2", "channel": "ops"}'
```
