# ailoop-js

TypeScript client for an [ailoop](https://github.com/goailoop/ailoop) server over HTTP and WebSocket.

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

## Client surface (summary)

- **Interactions:** `ask`, `authorize`, `say`, `navigate`, `respond`, `getMessage`
- **Tasks:** `createTask`, `updateTask`, `listTasks`, `getTask`, `addDependency`, `removeDependency`, `getReadyTasks`, `getBlockedTasks`, `getDependencyGraph`
- **WebSocket:** `connect`, `disconnect`, `subscribe`, `unsubscribe`
- **Health / version:** `checkHealth`, `checkVersion`, `ensureVersionCompatibility`

See `src/client.ts` for signatures and options.

## Compatibility

- Node.js: `>=16.0.0`
- TypeScript: `>=5`

## Contributing

Building and testing this package inside the workspace: [CONTRIBUTING.md](../CONTRIBUTING.md).
