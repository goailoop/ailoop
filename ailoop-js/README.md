# ailoop-js

TypeScript SDK for communicating with ailoop servers via HTTP and WebSocket protocols.

## Installation

```bash
npm install ailoop-js
```

## Quick Start

```typescript
import { AiloopClient } from 'ailoop-js';

// Create a client
const client = new AiloopClient({
  baseURL: 'http://localhost:8080'
});

// Check server health
const health = await client.checkHealth();
console.log('Server status:', health.status);

// Send a question
const response = await client.ask('general', 'What is the capital of France?');
console.log('Response:', response);

// Send a notification
await client.say('general', 'System maintenance scheduled', 'high');

// WebSocket connection (when implemented)
await client.connect();
await client.subscribe('general');
```

## Features

- **HTTP API**: Send messages via REST endpoints
- **WebSocket Support**: Real-time bidirectional communication
- **Type Safety**: Full TypeScript types matching Rust models
- **Version Compatibility**: Automatic server version checking
- **Error Handling**: Comprehensive error types and handling

## API Reference

### AiloopClient

#### Constructor

```typescript
new AiloopClient(options?: AiloopClientOptions)
```

#### Methods

- `ask(channel, question, timeout?, choices?)` - Send a question and wait for response
- `authorize(channel, action, timeout?, context?)` - Request authorization
- `say(channel, text, priority?)` - Send a notification
- `navigate(channel, url, context?)` - Send navigation request
- `getMessage(id)` - Retrieve message by ID
- `respond(messageId, answer, responseType?)` - Send response to a message
- `connect()` - Connect via WebSocket
- `disconnect()` - Disconnect WebSocket
- `subscribe(channel)` - Subscribe to channel
- `unsubscribe(channel)` - Unsubscribe from channel
- `checkHealth()` - Check server health
- `checkVersion()` - Check version compatibility

## Message Types

The SDK supports all ailoop message types:

- **Question**: Interactive questions with optional choices
- **Authorization**: Requests requiring approval
- **Notification**: Informational messages with priority levels
- **Response**: Answers to questions
- **Navigate**: URL navigation requests

## Development

```bash
# Install dependencies
npm install

# Run tests
npm test

# Build the project
npm run build

# Type checking
npm run type-check

# Lint code
npm run lint
```

## Compatibility

- **Node.js**: >=16.0.0
- **TypeScript**: >=5.0.0
- **Server**: ailoop >=0.1.1

## License

MIT OR Apache-2.0
