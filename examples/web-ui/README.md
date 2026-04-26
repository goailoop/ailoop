# ailoop web UI example

Minimal browser interface for monitoring channels and message activity from an ailoop server.

## Start

1. Run server:

```bash
ailoop serve
```

2. Open the UI:

- `http://127.0.0.1:8081`

The UI uses HTTP API on `8081` and WebSocket stream on `8080`.

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
- If updates stop, verify WebSocket port `8080` is reachable.
- If initial load fails, verify HTTP API on `8081`.

## Contributing

Use root workflow in `../../CONTRIBUTING.md`.
