# ailoop-server WebSocket Wire Protocol

This document describes the WebSocket protocol used by `ailoop-server`.
All communication happens over a single WebSocket connection at `ws://<host>:<port>/`
(or `ws://<host>:<port><base_path>/` when `ServeConfig.base_path` is set).

---

## Semver Policy

| Change type | Version bump required |
|---|---|
| New optional field added to any frame | None (additive, backward-compatible) |
| Existing field renamed or removed | Major version bump |
| New frame type added | Minor version bump |
| Frame type removed | Major version bump |
| Encoding changed (e.g. JSON → MessagePack) | Major version bump |

Clients SHOULD tolerate unknown fields in received frames (be liberal in what you accept).

---

## Connection Lifecycle

1. Client opens a WebSocket connection to the server root (`/` or `<base_path>/`).
2. The connection starts in **Agent mode** — it can send messages to be enqueued.
3. To switch to **Viewer mode**, the client sends a Hello frame (see below).
4. Viewer mode is read-only: the client receives all broadcast messages but its
   write frames are ignored.

---

## Hello Frame (Client → Server)

Sent by a viewer client immediately after the WebSocket handshake to subscribe to the
message stream. Sending this frame switches the connection to Viewer mode.

```json
{"subscribe": "*"}
```

Or to subscribe to specific channels only:

```json
{"subscribe": ["channel-a", "channel-b"]}
```

| Field | Type | Description |
|---|---|---|
| `subscribe` | `"*"` or `string[]` | Channels to subscribe to. `"*"` subscribes to all. |

After the server processes the Hello frame it replays up to 500 recent messages per
channel so the viewer page is not blank on connect.

---

## Message Envelope (Server → Viewer)

Each message broadcast to viewers is a JSON-serialized `Message` object.

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "channel": "default",
  "sender_type": "agent",
  "content": { ... },
  "timestamp": "2026-05-10T12:00:00Z",
  "metadata": null
}
```

| Field | Type | Description |
|---|---|---|
| `id` | UUID string | Unique message identifier |
| `channel` | string | Channel this message belongs to |
| `sender_type` | `"agent"` \| `"server"` \| `"human"` | Who produced this message |
| `content` | object | One of the `MessageContent` variants below |
| `timestamp` | ISO 8601 datetime | When the message was created |
| `metadata` | object \| null | Arbitrary application metadata |

---

## MessageContent Variants

All content objects include a `type` discriminant field.

### Decision

```json
{
  "type": "decision",
  "decision_id": "deploy-prod",
  "summary": "Deploy to production?",
  "context_markdown": "## Context\n...",
  "options": [
    {"id": "yes", "label": "Yes", "detail_markdown": null},
    {"id": "no",  "label": "No",  "detail_markdown": null}
  ],
  "recommendation": {"option_id": "yes", "rationale": "Tests passed"},
  "timeout_seconds": 60
}
```

### Authorization

```json
{
  "type": "authorization",
  "action": "rm -rf /tmp/build",
  "context_markdown": null,
  "timeout_seconds": 30
}
```

### Notification

```json
{
  "type": "notification",
  "text": "Build succeeded",
  "priority": "normal"
}
```

Priority values: `low`, `normal`, `high`, `urgent`.

### Response (server → viewer after human answers)

```json
{
  "type": "response",
  "answer": "yes",
  "response_type": "text"
}
```

`response_type` values: `text`, `authorization_approved`, `authorization_denied`,
`timeout`, `cancelled`.

### Navigate

```json
{
  "type": "navigate",
  "url": "https://example.com/report"
}
```

### TaskCreate / TaskUpdate / TaskDependencyAdd / TaskDependencyRemove

Task-related events. Exact shape mirrors the `Task` schema in `docs/openapi/ailoop-server.yaml`.

---

## Health Endpoint Response (stable shape)

`GET /api/v1/health` always returns:

```json
{
  "status": "ok",
  "version": "<semver>",
  "active_connections": 3,
  "queue_size": 0,
  "active_channels": 2
}
```

The fields `status`, `version`, `active_connections`, `queue_size`, and `active_channels`
are **stable** across all patch releases of `ailoop-server`. Additional fields may be
added in minor releases.

---

## Shutdown Drain Policy

When the server receives a shutdown signal (Ctrl+C or `CancellationToken::cancel()`):

1. The server stops accepting new TCP connections.
2. Background task loops observe the cancellation and exit after the current tick (≤ 100 ms).
3. In-flight messages already dequeued by the processing loop are allowed to complete.
4. Any `POST /api/v1/messages` arriving after cancellation returns `503 Service Unavailable`
   with body `{"error":"server is shutting down"}`.
