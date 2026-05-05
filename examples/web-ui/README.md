# ailoop web UI example

Minimal browser interface for monitoring channels and message activity from an ailoop server.

## Start

1. Run server with web UI enabled:

```bash
ailoop serve --web
```

2. Open the UI:

- `http://127.0.0.1:8080`

Both the HTTP API and the WebSocket stream are served from port `8080`.

## What you can do

- View active channels
- Inspect channel message history
- Watch live updates in real time
- Track basic broadcast and activity stats

## API endpoints used by UI

- `GET /api/channels`
- `GET /api/channels/:channel/messages`
- `GET /api/channels/:channel/stats`
- `GET /api/stats`

## Files

- `index.html`
- `styles.css`
- `app.js`

## Troubleshooting

- If no channels appear, verify agents are publishing to server.
- If updates stop, verify port `8080` is reachable.
- If initial load fails, verify the server is running on `8080`.

## Contributing

Use root workflow in `../../CONTRIBUTING.md`.
