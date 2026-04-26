# ailoop-js

TypeScript SDK for communicating with an ailoop server over HTTP/WebSocket-compatible endpoints.

## Install

```bash
npm install ailoop-js
```

## Quick start

```typescript
import { AiloopClient } from "ailoop-js";

const client = new AiloopClient({ baseURL: "http://127.0.0.1:8080" });

await client.say("public", "build finished", "normal");
const message = await client.ask("public", "approve release?", 60);
console.log(message.content);
```

## Core APIs

- `ask(channel, question, timeout?, choices?)`
- `authorize(channel, action, timeout?, context?)`
- `say(channel, text, priority?)`
- `navigate(channel, url, context?)`
- `respond(messageId, answer, responseType?)`
- `getMessage(id)`
- `checkHealth()`
- `checkVersion()`
- `connect()`, `disconnect()`, `subscribe(channel)`, `unsubscribe(channel)`

## Development

```bash
npm install
npm run lint
npm run type-check
npm test
npm run build
```

## Compatibility

- Node.js: `>=16.0.0`
- TypeScript: `>=5`

## Contributing

Use root workflow in `../CONTRIBUTING.md`.
