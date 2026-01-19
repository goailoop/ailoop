# ailoop-py

Python SDK for communicating with ailoop servers via HTTP and WebSocket protocols.

## Installation

```bash
pip install ailoop-py
```

For development:

```bash
git clone https://github.com/goailoop/ailoop-py.git
cd ailoop-py
pip install -e ".[dev]"
```

## Quick Start

```python
import asyncio
from ailoop import AiloopClient

async def main():
    async with AiloopClient("http://localhost:8080") as client:
        # Ask a question with multiple choices
        response = await client.ask(
            "What is the best color?",
            choices=["Red", "Blue", "Green"],
            timeout=30
        )

        # Send a notification
        await client.say("Task completed successfully!", priority="high")

        # Request authorization
        auth = await client.authorize("Deploy to production", timeout=300)
        if auth.content.response_type == "authorization_approved":
            print("✅ Deployment authorized")
        else:
            print("❌ Deployment denied")

asyncio.run(main())
```

## Features

- **HTTP REST API**: Synchronous operations for sending messages
- **WebSocket Support**: Real-time bidirectional communication
- **Type Safety**: Full type hints with Pydantic models
- **Async/Await**: Modern Python async support
- **Auto-reconnection**: Exponential backoff for WebSocket reconnection
- **Version Compatibility**: Automatic server version checking
- **Message Retrieval**: Get messages by ID and send responses
- **Event-driven**: WebSocket message handlers for real-time updates

## API Reference

### AiloopClient

Main client class for server communication.

#### Constructor

```python
AiloopClient(
    server_url="http://localhost:8080",
    channel="public",
    timeout=30.0,
    reconnect_attempts=5,
    reconnect_delay=1.0
)
```

#### Core Methods

##### Message Operations

- `ask(question, channel=None, timeout=60, choices=None)` → `Message`
  - Ask a question and get the sent message (responses come via WebSocket)

- `authorize(action, channel=None, timeout=300, context=None)` → `Message`
  - Request authorization for an action

- `say(message, channel=None, priority="normal")` → `Message`
  - Send a notification message

- `navigate(url, channel=None)` → `Message`
  - Send a navigation request

##### Data Retrieval

- `get_message(message_id)` → `Message`
  - Retrieve a message by its UUID

- `respond(original_message_id, answer=None, response_type="text")` → `Message`
  - Send a response to an existing message

##### WebSocket Management

- `connect_websocket()` → `None`
  - Connect to WebSocket for real-time updates

- `disconnect_websocket()` → `None`
  - Disconnect from WebSocket

- `subscribe_to_channel(channel)` → `None`
  - Subscribe to a channel for real-time messages

- `unsubscribe_from_channel(channel)` → `None`
  - Unsubscribe from a channel

- `add_message_handler(handler)` → `None`
  - Add a callback for incoming WebSocket messages

##### Utilities

- `check_version_compatibility()` → `Dict`
  - Check server version compatibility

### Models

#### Core Types

- **`Message`**: Core message structure with ID, channel, sender, content, timestamp
- **`MessageContent`**: Union type for different message content types
- **`SenderType`**: `AGENT` or `HUMAN`
- **`ResponseType`**: `TEXT`, `AUTHORIZATION_APPROVED`, `AUTHORIZATION_DENIED`, `TIMEOUT`, `CANCELLED`
- **`NotificationPriority`**: `LOW`, `NORMAL`, `HIGH`, `URGENT`

#### Content Types

- **`QuestionContent`**: Questions with text, timeout, and optional choices
- **`AuthorizationContent`**: Authorization requests with action and context
- **`NotificationContent`**: Notifications with text and priority
- **`ResponseContent`**: Responses with answer and type
- **`NavigateContent`**: Navigation requests with URL

### Exceptions

- **`AiloopError`**: Base exception for all ailoop errors
- **`ConnectionError`**: Network or server connection issues
- **`ValidationError`**: Invalid message data or server responses
- **`TimeoutError`**: Operation timeouts

## Advanced Usage

### WebSocket Real-time Updates

```python
import asyncio
from ailoop import AiloopClient

async def message_handler(message):
    """Handle incoming messages."""
    print(f"Received: {message}")
    if message.get("content", {}).get("type") == "response":
        print(f"Response: {message['content']['answer']}")

async def main():
    async with AiloopClient("ws://localhost:8080") as client:
        # Add message handler
        client.add_message_handler(message_handler)

        # Subscribe to channel
        await client.subscribe_to_channel("my-channel")

        # Ask a question - responses will come via WebSocket
        question = await client.ask("What's the weather?", channel="my-channel")

        # Wait for responses
        await asyncio.sleep(60)

asyncio.run(main())
```

### Error Handling

```python
from ailoop import AiloopClient
from ailoop.exceptions import ConnectionError, ValidationError

async def robust_client():
    try:
        async with AiloopClient("http://localhost:8080") as client:
            response = await client.ask("Test question")
            print(f"Success: {response.id}")
    except ConnectionError as e:
        print(f"Connection failed: {e}")
    except ValidationError as e:
        print(f"Invalid data: {e}")
```

### Custom Message Types

```python
from ailoop.models import Message, QuestionContent, SenderType
from datetime import datetime
from uuid import uuid4

# Create custom message
custom_message = Message(
    id=uuid4(),
    channel="custom",
    sender_type=SenderType.AGENT,
    content=QuestionContent(
        text="Custom question",
        timeout_seconds=120
    ),
    timestamp=datetime.utcnow()
)
```

## Testing

Run the test suite:

```bash
pytest
```

Run with coverage:

```bash
pytest --cov=ailoop --cov-report=html
```

## Development

### Project Structure

```
ailoop-py/
├── src/ailoop/
│   ├── __init__.py      # Package exports
│   ├── client.py        # Main client implementation
│   ├── models.py        # Data models
│   └── exceptions.py    # Custom exceptions
├── tests/
│   ├── test_models.py   # Model tests
│   └── test_client.py   # Client tests
├── pyproject.toml       # Package configuration
└── README.md           # This file
```

### Building

```bash
# Build distribution
python -m build

# Install locally
pip install -e .
```

## Compatibility

- **Python**: 3.8+
- **Dependencies**: httpx, websockets, pydantic v2+
- **Server**: ailoop v0.1.1+

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT OR Apache-2.0
