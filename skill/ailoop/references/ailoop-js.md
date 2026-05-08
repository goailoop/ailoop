# ailoop-js Reference

TypeScript/JavaScript SDK for ailoop server communication. Client-only -- requires a running `ailoop serve` instance.

## Installation

```bash
npm install ailoop-js
```

Dependencies: `axios ^1.6`, `isomorphic-ws ^5.0`, `ws ^8.14`. Requires Node.js >= 16.

## Setup

```typescript
import { AiloopClient } from 'ailoop-js';

const client = new AiloopClient({
  baseURL: 'http://localhost:8080',
  timeout: 30000,
  maxRetries: 5,
  retryDelay: 1000,
});
```

### AiloopClientOptions

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `baseURL` | `string` | `'http://localhost:8080'` | Server base URL |
| `timeout` | `number` | `30000` | HTTP request timeout (ms) |
| `maxRetries` | `number` | `5` | Max WebSocket reconnection attempts |
| `retryDelay` | `number` | `1000` | Base reconnection delay (ms), exponential backoff |

## Sending Messages

All send methods POST to `/api/v1/messages` and return `Promise<Message>`.

### ask -- Ask a question

```typescript
const msg = await client.ask(
  'general',
  'What approach should we use?',
  60,
  ['Option A', 'Option B', 'Option C']
);
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `channel` | `string` | required | Target channel |
| `question` | `string` | required | Question text |
| `timeoutSeconds?` | `number` | `60` | Response timeout in seconds |
| `choices?` | `string[]` | `undefined` | Multiple choice options |

### authorize -- Request authorization

```typescript
const msg = await client.authorize(
  'admin-ops',
  'Deploy v2.0 to production',
  300,
  { version: '2.0', environment: 'prod' }
);
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `channel` | `string` | required | Target channel |
| `action` | `string` | required | Action description |
| `timeoutSeconds?` | `number` | `300` | Timeout (denied on expiry) |
| `context?` | `Record<string, any>` | `undefined` | Additional metadata |

### say -- Send a notification

```typescript
const msg = await client.say('monitoring', 'Build completed', 'high');
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `channel` | `string` | required | Target channel |
| `text` | `string` | required | Notification text |
| `priority?` | `NotificationPriority` | `'normal'` | `'low'`, `'normal'`, `'high'`, `'urgent'` |

### navigate -- Send a navigation URL

```typescript
const msg = await client.navigate('public', 'https://dashboard.example.com');
```

### respond -- Reply to a message

Fetches the original message to determine channel, then sends response with `correlation_id`.

```typescript
const response = await client.respond(
  '550e8400-e29b-41d4-a716-446655440000',
  'Yes, proceed',
  'text'
);
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `messageId` | `string` | required | Original message ID |
| `answer?` | `string` | `undefined` | Response text |
| `responseType?` | `ResponseType` | `'text'` | `'text'`, `'authorization_approved'`, `'authorization_denied'`, `'timeout'`, `'cancelled'` |

### getMessage -- Retrieve a message

```typescript
const msg = await client.getMessage('550e8400-e29b-41d4-a716-446655440000');
```

## Listening for Messages (WebSocket)

### Connect and register handlers

```typescript
import { AiloopClient, WebSocketMessage } from 'ailoop-js';

const client = new AiloopClient({ baseURL: 'http://localhost:8080' });

client.addMessageHandler((message: WebSocketMessage) => {
  if (message.type === 'message' && message.data) {
    const content = message.data.content;
    switch (content.type) {
      case 'question':
        console.log(`Question: ${content.text}`);
        break;
      case 'authorization':
        console.log(`Auth request: ${content.action}`);
        break;
      case 'notification':
        console.log(`Notification: ${content.text}`);
        break;
      case 'response':
        console.log(`Response: ${content.answer}`);
        break;
    }
  }
});

client.addConnectionHandler((event) => {
  console.log(`Connection: ${event.type}`);
});

await client.connect();               // version check + WebSocket
await client.subscribe('public');

// Keep process alive
await new Promise(() => {});
```

### Channel subscriptions

```typescript
await client.subscribe('dev-review');
await client.unsubscribe('dev-review');
```

Subscriptions are automatically re-established on reconnection.

### Reconnection

Automatic exponential backoff: `delay = 1000ms * 2^(attempt-1)`. Max 5 attempts. Configured by `maxRetries` and `retryDelay` constructor options.

### Connection state

```typescript
const state = client.getConnectionState();
// { connected: boolean, url?: string, channels: string[] }
```

### Disconnect

```typescript
await client.disconnect();
```

Sets `manualDisconnect` flag, clears reconnect timers, closes WebSocket, fires `'disconnected'` to handlers.

### Important limitations

- `ask()` and `authorize()` return the **sent** message, not the human's reply. Use `addMessageHandler()` and match on `correlation_id` to get responses.
- Handlers receive `WebSocketMessage` objects. There is no typed event dispatch.
- No `removeMessageHandler()` or `removeConnectionHandler()` -- create a new client to clear handlers.

## Task Management

### Create a task

```typescript
const task = await client.createTask(
  'ops',
  'Deploy service',
  'Deploy v2 to staging',
  'alice',
  { priority: 'high' }
);
```

### Update task state

```typescript
const task = await client.updateTask('ops', 'task-id-123', 'done');
```

Valid states: `'pending'`, `'done'`, `'abandoned'`.

### List / get tasks

```typescript
const tasks = await client.listTasks('ops', 'pending');
const task = await client.getTask('task-id-123');
```

### Dependencies

```typescript
await client.addDependency('child-id', 'parent-id', 'blocks');
await client.removeDependency('child-id', 'parent-id');
```

Dependency types: `'blocks'`, `'related'`, `'parent'`.

### Query dependency state

```typescript
const ready = await client.getReadyTasks('ops');
const blocked = await client.getBlockedTasks('ops');
const graph = await client.getDependencyGraph('task-id-123');
// graph: { task: Task; parents: Task[]; children: Task[] }
```

## Health & Version

```typescript
const health = await client.checkHealth();
// { status, version, activeConnections, queueSize, activeChannels }

const versionInfo = await client.checkVersion();
// { clientVersion, serverVersion, compatible, warnings[], errors[] }

await client.ensureVersionCompatibility(); // throws ValidationError if incompatible
```

## MessageFactory

Static factory for creating partial message objects (without `id` and `timestamp`, which are server-assigned):

```typescript
import { MessageFactory } from 'ailoop-js';

const q = MessageFactory.createQuestion('general', 'Ready?', 60, ['yes', 'no']);
const a = MessageFactory.createAuthorization('admin', 'Deploy', 300);
const n = MessageFactory.createNotification('general', 'Done', 'normal');
const r = MessageFactory.createResponse(correlationId, 'yes', 'text');
const nav = MessageFactory.createNavigate('general', 'https://example.com');
```

## Types

### Message

```typescript
interface Message {
  id: string;
  channel: string;
  sender_type: SenderType;
  content: MessageContent;
  timestamp: string;
  correlation_id?: string;
  metadata?: Record<string, any>;
}
```

### Content types (discriminated union on `type`)

| Type | Key fields |
|------|------------|
| `question` | `text`, `timeout_seconds`, `choices?` |
| `authorization` | `action`, `timeout_seconds`, `context?` |
| `notification` | `text`, `priority` |
| `response` | `answer?`, `response_type` |
| `navigate` | `url` |

### Type aliases

| Name | Values |
|------|--------|
| `SenderType` | `'HUMAN'`, `'AGENT'`, `'SYSTEM'` |
| `ResponseType` | `'text'`, `'authorization_approved'`, `'authorization_denied'`, `'timeout'`, `'cancelled'` |
| `NotificationPriority` | `'low'`, `'normal'`, `'high'`, `'urgent'` |
| `TaskState` | `'pending'`, `'done'`, `'abandoned'` |
| `DependencyType` | `'blocks'`, `'related'`, `'parent'` |

### Handler types

```typescript
type MessageHandler = (message: WebSocketMessage) => void | Promise<void>;
type ConnectionHandler = (event: { type: 'connected' | 'disconnected' | 'error'; error?: string }) => void | Promise<void>;
```

### WebSocketMessage

```typescript
interface WebSocketMessage {
  type: 'message' | 'subscribe' | 'unsubscribe' | 'connected' | 'disconnected' | 'error';
  channel?: string;
  data?: any;
  error?: string;
}
```

### Task

```typescript
interface Task {
  id: string;
  title: string;
  description: string;
  state: TaskState;
  created_at: string;
  updated_at: string;
  assignee?: string;
  metadata?: Record<string, any>;
  depends_on: string[];
  blocking_for: string[];
  blocked: boolean;
  dependency_type?: DependencyType;
}
```

## Error Classes

All inherit from `AiloopError` which extends `Error`:

| Class | `code` | When thrown |
|-------|--------|-------------|
| `ConnectionError` | `'CONNECTION_ERROR'` | HTTP/network failures, WebSocket failures |
| `ValidationError` | `'VALIDATION_ERROR'` | Invalid input, 400 responses, 404 not-found |
| `TimeoutError` | `'TIMEOUT_ERROR'` | Request timeout |

## REST API Endpoints

| Endpoint | Method | Client method(s) |
|----------|--------|-------------------|
| `/api/v1/messages` | POST | `ask`, `authorize`, `say`, `navigate` |
| `/api/v1/messages/{id}` | GET | `getMessage`, `respond` |
| `/api/v1/tasks` | POST | `createTask` |
| `/api/v1/tasks` | GET | `listTasks` |
| `/api/v1/tasks/{id}` | GET/PUT | `getTask`, `updateTask` |
| `/api/v1/tasks/{id}/dependencies` | POST | `addDependency` |
| `/api/v1/tasks/{id}/dependencies/{id}` | DELETE | `removeDependency` |
| `/api/v1/tasks/ready` | GET | `getReadyTasks` |
| `/api/v1/tasks/blocked` | GET | `getBlockedTasks` |
| `/api/v1/tasks/{id}/graph` | GET | `getDependencyGraph` |
| `/api/v1/health` | GET | `checkHealth`, `checkVersion` |
| `ws://{host}/ws` | WebSocket | `connect`, `subscribe`, `unsubscribe` |
