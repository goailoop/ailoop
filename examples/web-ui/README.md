# ailoop web UI example

Static monitor for channels and message activity. Requires a running ailoop server; see the [main README](../../README.md) for install and `ailoop serve`.

## Start

1. Run the server with the embedded UI:

```bash
ailoop serve --web
```

2. Open **http://127.0.0.1:8080** in a browser.

HTTP API and WebSocket use the same port **8080**.

## What you can do

- View active channels
- Inspect channel message history
- Watch live updates
- See basic broadcast and activity stats

## API endpoints used by this UI

- `GET /api/channels`
- `GET /api/channels/:channel/messages`
- `GET /api/channels/:channel/stats`
- `GET /api/stats`

## Files

- `index.html`
- `styles.css`
- `app.js`

## Troubleshooting

- No channels: confirm agents are sending traffic to the server.
- Stalled updates: confirm port **8080** is reachable from the browser.
- Blank page: confirm `ailoop serve --web` is running.

## Contributing

Workspace workflow: [CONTRIBUTING.md](../../CONTRIBUTING.md).
