# ailoop Web UI

A real-time web interface for monitoring AI agent activity through the ailoop server.

## Features

- **Real-time message streaming** via WebSocket
- **Channel-based organization** of agent messages
- **Live statistics** showing server activity
- **Formatted message display** with agent identification
- **Automatic reconnection** on connection loss
- **Responsive design** for desktop and mobile

## Setup

1. Start the ailoop server:
   ```bash
   ailoop serve
   ```

2. The web UI will be available at: `http://localhost:8081`
   (Note: The API runs on port 8081, WebSocket on 8080)

3. Open the web UI in your browser and start monitoring agent activity.

## Usage

### Monitoring Agents

1. **Connect**: The web UI automatically connects to the ailoop server via WebSocket
2. **View Channels**: Active channels appear in the sidebar with message counts
3. **Select Channel**: Click on any channel to view its messages
4. **Real-time Updates**: New messages appear automatically as agents send them
5. **Statistics**: View live server statistics in the sidebar

### Message Display

Messages are formatted with:
- **Agent type** in brackets (e.g., `[cursor]`, `[claude]`)
- **Timestamp** in local time format
- **Message type** indicated by icons and colors:
  - 🟢 Notifications (normal priority)
  - 🟡 High priority notifications
  - 🔴 Urgent/error notifications
  - ❓ Questions
  - 🔐 Authorization requests
  - 📤 Responses

### Controls

- **Refresh**: Reload channels and statistics
- **Clear**: Clear the current message display
- **Tab switching**: Click on different channels to view their messages

## API Endpoints

The web UI uses the following ailoop API endpoints:

- `GET /api/channels` - List all active channels
- `GET /api/channels/:channel/messages` - Get messages for a channel
- `GET /api/channels/:channel/stats` - Get statistics for a channel
- `GET /api/stats` - Get overall broadcast statistics
- `WebSocket` on port 8080 for real-time message streaming

## Development

### Files

- `index.html` - Main HTML structure
- `styles.css` - Responsive CSS styling
- `app.js` - JavaScript client with WebSocket and API integration
- `README.md` - This documentation

### Adding Features

The web UI is built with vanilla JavaScript for simplicity. To add new features:

1. **API Integration**: Add new API calls in `app.js`
2. **UI Components**: Add HTML elements and style them in `styles.css`
3. **Real-time Updates**: Extend WebSocket message handling
4. **Channel Management**: Update channel selection logic

### Browser Support

- Modern browsers with WebSocket support
- Chrome, Firefox, Safari, Edge
- Mobile browsers (responsive design)

## Troubleshooting

### Connection Issues

- **WebSocket fails**: Check that ailoop server is running on port 8080
- **API fails**: Check that server is accessible on port 8081
- **Reconnection**: The UI automatically attempts to reconnect on failure

### Performance

- **High message volume**: The UI displays the most recent 50 messages per channel
- **Memory usage**: Old messages are replaced as new ones arrive
- **Scrolling**: Auto-scroll to newest messages (can be disabled)

### Common Issues

- **No channels visible**: Ensure agents are connected and sending messages
- **Messages not updating**: Check WebSocket connection status
- **Slow loading**: Check network connectivity to server
