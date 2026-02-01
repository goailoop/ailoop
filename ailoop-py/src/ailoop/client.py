"""Ailoop client for HTTP and WebSocket communication."""

from __future__ import annotations

import asyncio
import json
import logging
import uuid
from datetime import datetime
from typing import Any, Callable, Dict, List, Optional, Type, Union, cast
from types import TracebackType
from uuid import UUID

import httpx
import websockets

from .exceptions import ConnectionError, TimeoutError
from .exceptions import ValidationError as AiloopValidationError
from .models import Message, NavigateContent, NotificationPriority, ResponseType, SenderType
from .models import Task

logger = logging.getLogger(__name__)


class AiloopClient:
    """Client for communicating with ailoop servers.

    Supports both HTTP REST API and WebSocket real-time communication.
    """

    def __init__(
        self,
        server_url: str = "http://localhost:8080",
        channel: str = "public",
        timeout: float = 30.0,
        reconnect_attempts: int = 5,
        reconnect_delay: float = 1.0,
    ):
        """Initialize the ailoop client.

        Args:
            server_url: Base URL of the ailoop server
            channel: Default channel for messages
            timeout: Default timeout for operations in seconds
            reconnect_attempts: Maximum WebSocket reconnection attempts
            reconnect_delay: Delay between reconnection attempts in seconds
        """
        self.server_url = server_url.rstrip("/")
        self.channel = channel
        self.timeout = timeout
        self.reconnect_attempts = reconnect_attempts
        self.reconnect_delay = reconnect_delay

        # HTTP client
        self._http_client: Optional[httpx.AsyncClient] = None

        # WebSocket connection
        self._websocket: Optional[Any] = None  # websockets.WebSocketClientProtocol
        self._websocket_task: Optional[asyncio.Task] = None
        self._reconnect_attempts = 0
        self._subscribed_channels: set[str] = set()

        # Event handlers
        self._message_handlers: List[Callable] = []
        self._connection_handlers: List[Callable] = []

    async def __aenter__(self) -> "AiloopClient":
        """Async context manager entry."""
        await self.connect()
        await self.connect_websocket()
        return self

    async def __aexit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc_val: Optional[BaseException],
        exc_tb: Optional[TracebackType],
    ) -> None:
        """Async context manager exit."""
        await self.disconnect_websocket()
        await self.disconnect()

    async def connect(self) -> None:
        """Connect to the ailoop server."""
        # Initialize HTTP client
        self._http_client = httpx.AsyncClient(
            base_url=self.server_url,
            timeout=self.timeout,
        )

        # Test connection and check version compatibility
        try:
            version_info = await self.check_version_compatibility()
            if not version_info["compatible"]:
                logger.warning(
                    f"Version mismatch: Client v{version_info['client_version']} "
                    f"vs Server v{version_info['server_version']}"
                )
            else:
                logger.info(f"Connected to ailoop server v{version_info['server_version']}")
        except Exception as e:
            raise ConnectionError(f"Failed to connect to ailoop server: {e}") from e

    async def disconnect(self) -> None:
        """Disconnect from the ailoop server."""
        # Close WebSocket connection
        if self._websocket_task:
            self._websocket_task.cancel()
            try:
                await self._websocket_task
            except asyncio.CancelledError:
                pass
            self._websocket_task = None

        if self._websocket:
            await self._websocket.close()
            self._websocket = None

        # Close HTTP client
        if self._http_client:
            await self._http_client.aclose()
            self._http_client = None

    async def ask(
        self,
        question: str,
        channel: Optional[str] = None,
        timeout: Optional[int] = None,
        choices: Optional[List[str]] = None,
    ) -> Message:
        """Ask a question and wait for a response.

        Args:
            question: The question text
            channel: Channel to send to (default: client default)
            timeout: Response timeout in seconds (default: 60)
            choices: Multiple choice options

        Returns:
            Response message

        Raises:
            ConnectionError: If server connection fails
            TimeoutError: If response times out
            ValidationError: If message validation fails
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel
        timeout = timeout or 60

        # Create question message
        message = Message.create_question(
            channel=channel,
            text=question,
            timeout_seconds=timeout,
            choices=choices,
        )

        # Send message via HTTP API
        sent_message = await self._send_message(message)

        # If WebSocket is connected, also send via WebSocket for real-time responses
        if self._websocket:
            try:
                # Subscribe to the channel if not already subscribed
                if sent_message.channel not in self._subscribed_channels:
                    await self.subscribe_to_channel(sent_message.channel)
            except Exception as e:
                logger.warning(f"Failed to subscribe to channel via WebSocket: {e}")

        return sent_message

    async def authorize(
        self,
        action: str,
        channel: Optional[str] = None,
        timeout: Optional[int] = None,
        context: Optional[Dict[str, Any]] = None,
    ) -> Message:
        """Request authorization for an action.

        Args:
            action: Description of the action requiring authorization
            channel: Channel to send to (default: client default)
            timeout: Authorization timeout in seconds (default: 300)
            context: Additional context for the authorization

        Returns:
            Authorization response message
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel
        timeout = timeout or 300

        # Create authorization message
        message = Message.create_authorization(
            channel=channel,
            action=action,
            timeout_seconds=timeout,
            context=context,
        )

        # Send message via HTTP API
        sent_message = await self._send_message(message)

        # For HTTP-only implementation, return the sent message
        # WebSocket implementation (TASK-018) will add real-time response listening
        return sent_message

    async def say(
        self,
        message: str,
        channel: Optional[str] = None,
        priority: NotificationPriority = NotificationPriority.NORMAL,
    ) -> Message:
        """Send a notification message.

        Args:
            message: Notification text
            channel: Channel to send to (default: client default)
            priority: Message priority

        Returns:
            The sent notification message
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel

        # Create notification message
        notification = Message.create_notification(
            channel=channel,
            text=message,
            priority=priority,
        )

        # Send message via HTTP API
        return await self._send_message(notification)

    async def navigate(self, url: str, channel: Optional[str] = None) -> Message:
        """Send a navigation request.

        Args:
            url: URL to navigate to
            channel: Channel to send to (default: client default)

        Returns:
            The sent navigation message
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel

        # Create navigation message
        navigation = Message(
            id=UUID(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=NavigateContent(url=url),
            timestamp=datetime.utcnow(),
        )

        # Send message via HTTP API
        return await self._send_message(navigation)

    async def get_message(self, message_id: Union[str, UUID]) -> Message:
        """Get a message by its ID.

        Args:
            message_id: The message ID (string or UUID)

        Returns:
            The message

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If message not found
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        # Convert string to UUID if needed
        if isinstance(message_id, str):
            message_id = UUID(message_id)

        try:
            response = await self._http_client.get(f"/api/v1/messages/{message_id}")
            response.raise_for_status()

            response_data = response.json()
            return Message(**response_data)

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise AiloopValidationError(f"Message not found: {message_id}") from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to get message: {e}") from e

    async def respond(
        self,
        original_message_id: Union[str, UUID],
        answer: Optional[str] = None,
        response_type: ResponseType = ResponseType.TEXT,
    ) -> Message:
        """Send a response to a message.

        Args:
            original_message_id: ID of the message to respond to
            answer: Response text (for TEXT responses)
            response_type: Type of response

        Returns:
            The response message

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If message validation fails
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        # Convert string to UUID if needed
        if isinstance(original_message_id, str):
            original_message_id = UUID(original_message_id)

        # First get the original message to know the channel
        original_message = await self.get_message(original_message_id)

        # Create response message
        response = Message.create_response(
            channel=original_message.channel,
            correlation_id=original_message_id,
            answer=answer,
            response_type=response_type,
        )

        # Send response via HTTP API
        return await self._send_message(response)

    async def connect_websocket(self) -> None:
        """Connect to WebSocket for real-time communication."""
        if self._websocket_task and not self._websocket_task.done():
            return  # Already connected

        websocket_url = self.server_url.replace("http", "ws") + "/ws"
        logger.info(f"Connecting to WebSocket: {websocket_url}")

        self._websocket_task = asyncio.create_task(self._websocket_loop(websocket_url))

    async def disconnect_websocket(self) -> None:
        """Disconnect from WebSocket."""
        if self._websocket_task:
            self._websocket_task.cancel()
            try:
                await self._websocket_task
            except asyncio.CancelledError:
                pass
            self._websocket_task = None

        if self._websocket:
            await self._websocket.close()
            self._websocket = None

    async def subscribe_to_channel(self, channel: str) -> None:
        """Subscribe to a channel for real-time updates."""
        if not self._websocket:
            raise ConnectionError("WebSocket not connected")

        subscribe_msg = {"type": "subscribe", "channel": channel}
        await self._websocket.send(json.dumps(subscribe_msg))
        self._subscribed_channels.add(channel)
        logger.info(f"Subscribed to channel: {channel}")

    async def unsubscribe_from_channel(self, channel: str) -> None:
        """Unsubscribe from a channel."""
        if not self._websocket:
            raise ConnectionError("WebSocket not connected")

        unsubscribe_msg = {"type": "unsubscribe", "channel": channel}
        await self._websocket.send(json.dumps(unsubscribe_msg))
        self._subscribed_channels.discard(channel)
        logger.info(f"Unsubscribed from channel: {channel}")

    def add_message_handler(self, handler: Callable) -> None:
        """Add a handler for incoming messages."""
        self._message_handlers.append(handler)

    def add_connection_handler(self, handler: Callable) -> None:
        """Add a handler for connection events."""
        self._connection_handlers.append(handler)

    async def _websocket_loop(self, url: str) -> None:
        """Main WebSocket connection loop with reconnection."""
        while True:
            try:
                async with websockets.connect(url) as websocket:
                    self._websocket = websocket
                    self._reconnect_attempts = 0

                    # Notify connection handlers
                    for handler in self._connection_handlers:
                        try:
                            await handler({"type": "connected"})
                        except Exception as e:
                            logger.error(f"Connection handler error: {e}")

                    # Resubscribe to channels
                    for channel in self._subscribed_channels:
                        subscribe_msg = {"type": "subscribe", "channel": channel}
                        await websocket.send(json.dumps(subscribe_msg))

                    # Message handling loop
                    async for message in websocket:
                        try:
                            data = json.loads(message)
                            # Notify message handlers
                            for handler in self._message_handlers:
                                try:
                                    await handler(data)
                                except Exception as e:
                                    logger.error(f"Message handler error: {e}")
                        except json.JSONDecodeError as e:
                            logger.error(f"Invalid WebSocket message: {e}")

            except websockets.exceptions.ConnectionClosed:
                logger.warning("WebSocket connection closed")
            except Exception as e:
                logger.error(f"WebSocket error: {e}")

            # Reconnection logic
            if self._reconnect_attempts < self.reconnect_attempts:
                self._reconnect_attempts += 1
                delay = self.reconnect_delay * (2 ** (self._reconnect_attempts - 1))
                logger.info(f"Reconnecting in {delay} seconds (attempt {self._reconnect_attempts})")
                await asyncio.sleep(delay)
            else:
                logger.error("Max reconnection attempts reached")
                break

    async def check_version_compatibility(self) -> Dict[str, Any]:
        """Check server version compatibility.

        Returns:
            Dict with server version info and compatibility status

        Raises:
            ConnectionError: If server connection fails
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        try:
            response = await self._http_client.get("/api/v1/health")
            response.raise_for_status()

            health_data = response.json()
            server_version = health_data.get("version", "0.0.0")
            client_version = "0.1.1"  # Should match __version__

            # Simple version compatibility check
            # For now, just check if major versions match
            server_major = server_version.split(".")[0] if "." in server_version else "0"
            client_major = client_version.split(".")[0] if "." in client_version else "0"

            is_compatible = server_major == client_major

            return {
                "server_version": server_version,
                "client_version": client_version,
                "compatible": is_compatible,
                "health_data": health_data,
            }

        except Exception as e:
            raise ConnectionError(f"Failed to check version compatibility: {e}") from e

    async def create_task(
        self,
        title: str,
        description: str,
        channel: Optional[str] = None,
        assignee: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Task:
        """Create a new task.

        Args:
            title: Task title
            description: Task description
            channel: Channel to create task in (default: client default)
            assignee: Optional assignee for the task
            metadata: Optional task metadata

        Returns:
            The created Task

        Raises:
            ConnectionError: If server connection fails
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel

        from .models import Task, TaskState

        task_id = str(uuid.uuid4())
        Task(
            id=task_id,
            title=title,
            description=description,
            state=TaskState.PENDING,
            created_at=datetime.utcnow(),
            updated_at=datetime.utcnow(),
            assignee=assignee,
            metadata=metadata,
        )

        try:
            response = await self._http_client.post(
                "/api/v1/tasks",
                json={
                    "title": title,
                    "description": description,
                    "channel": channel,
                    "assignee": assignee,
                    "metadata": metadata,
                },
            )
            response.raise_for_status()

            response_data = response.json()
            return Task(**response_data)

        except httpx.HTTPStatusError as e:
            raise ConnectionError(f"HTTP error {e.response.status_code}: {e.response.text}") from e
        except Exception as e:
            raise ConnectionError(f"Failed to create task: {e}") from e

    async def update_task(
        self,
        task_id: str,
        state: str,
    ) -> Task:
        """Update a task's state.

        Args:
            task_id: Task ID
            state: New state (pending, done, abandoned)

        Returns:
            The updated Task

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If state is invalid
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        from .models import TaskState

        try:
            TaskState(state.lower())
        except ValueError:
            raise AiloopValidationError(
                f"Invalid state: {state}. Must be pending, done, or abandoned"
            )

        try:
            response = await self._http_client.put(
                f"/api/v1/tasks/{task_id}",
                json={"state": state},
            )
            response.raise_for_status()

            response_data = response.json()
            return Task(**response_data)

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise AiloopValidationError(f"Task not found: {task_id}") from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to update task: {e}") from e

    async def list_tasks(
        self,
        channel: Optional[str] = None,
        state: Optional[str] = None,
    ) -> List[Task]:
        """List tasks in a channel.

        Args:
            channel: Channel to list tasks from (default: client default)
            state: Optional filter by task state

        Returns:
            List of Tasks

        Raises:
            ConnectionError: If server connection fails
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel

        from .models import Task

        params: Dict[str, str] = {"channel": channel}
        if state:
            params["state"] = state.lower()

        try:
            response = await self._http_client.get("/api/v1/tasks", params=params)
            response.raise_for_status()

            response_data = response.json()
            return [Task(**task) for task in response_data.get("tasks", [])]

        except httpx.HTTPStatusError as e:
            raise ConnectionError(f"HTTP error {e.response.status_code}: {e.response.text}") from e
        except Exception as e:
            raise ConnectionError(f"Failed to list tasks: {e}") from e

    async def get_task(self, task_id: str) -> Task:
        """Get a task by ID.

        Args:
            task_id: Task ID

        Returns:
            The Task

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If task not found
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        from .models import Task

        try:
            response = await self._http_client.get(f"/api/v1/tasks/{task_id}")
            response.raise_for_status()

            response_data = response.json()
            return Task(**response_data)

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise AiloopValidationError(f"Task not found: {task_id}") from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to get task: {e}") from e

    async def add_dependency(
        self,
        task_id: str,
        depends_on: str,
        type: str = "blocks",
        channel: Optional[str] = None,
    ) -> None:
        """Add a dependency between tasks.

        Args:
            task_id: Child task ID
            depends_on: Parent task ID
            type: Dependency type (blocks, related, parent)
            channel: Channel containing tasks (default: client default)

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If dependency is invalid
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        from .models import DependencyType

        try:
            DependencyType(type.lower())
        except ValueError:
            raise AiloopValidationError(
                f"Invalid dependency type: {type}. Must be blocks, related, or parent"
            )

        channel = channel or self.channel

        try:
            response = await self._http_client.post(
                f"/api/v1/tasks/{task_id}/dependencies",
                json={
                    "child_id": str(task_id),
                    "parent_id": str(depends_on),
                    "dependency_type": type,
                },
            )
            response.raise_for_status()

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 400:
                raise AiloopValidationError(f"Invalid dependency: {e.response.text}") from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to add dependency: {e}") from e

    async def remove_dependency(
        self,
        task_id: str,
        depends_on: str,
        channel: Optional[str] = None,
    ) -> None:
        """Remove a dependency between tasks.

        Args:
            task_id: Child task ID
            depends_on: Parent task ID
            channel: Channel containing tasks (default: client default)

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If dependency not found
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel

        try:
            response = await self._http_client.delete(
                f"/api/v1/tasks/{task_id}/dependencies/{depends_on}",
            )
            response.raise_for_status()

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise AiloopValidationError(
                    f"Dependency not found between {task_id} and {depends_on}"
                ) from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to remove dependency: {e}") from e

    async def get_ready_tasks(
        self,
        channel: Optional[str] = None,
    ) -> List:
        """Get tasks that are ready to start (no blockers).

        Args:
            channel: Channel to query (default: client default)

        Returns:
            List of ready Tasks

        Raises:
            ConnectionError: If server connection fails
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        from .models import Task

        channel = channel or self.channel

        try:
            response = await self._http_client.get(
                "/api/v1/tasks/ready",
                params={"channel": channel},
            )
            response.raise_for_status()

            response_data = response.json()
            return [Task(**task) for task in response_data.get("tasks", [])]

        except httpx.HTTPStatusError as e:
            raise ConnectionError(f"HTTP error {e.response.status_code}: {e.response.text}") from e
        except Exception as e:
            raise ConnectionError(f"Failed to get ready tasks: {e}") from e

    async def get_dependency_graph(
        self,
        task_id: str,
        channel: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Get dependency graph for a task.

        Args:
            task_id: Task ID
            channel: Channel containing tasks (default: client default)

        Returns:
            Dictionary with task, parents, and children

        Raises:
            ConnectionError: If server connection fails
            ValidationError: If task not found
        """
        if not self._http_client:
            raise ConnectionError("Client not connected")

        channel = channel or self.channel

        try:
            response = await self._http_client.get(f"/api/v1/tasks/{task_id}/graph")
            response.raise_for_status()

            return cast(Dict[str, Any], response.json())

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise AiloopValidationError(f"Task not found: {task_id}") from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to get dependency graph: {e}") from e

    async def _send_message(self, message: Message) -> Message:
        """Send a message via HTTP API and return the sent message."""
        if not self._http_client:
            raise ConnectionError("Client not connected")

        try:
            response = await self._http_client.post(
                "/api/v1/messages",
                json=message.dict(),
            )
            response.raise_for_status()

            # The server returns the created message
            response_data = response.json()
            return Message(**response_data)

        except httpx.TimeoutException as e:
            raise TimeoutError(f"Request timed out: {e}") from e
        except httpx.HTTPStatusError as e:
            if e.response.status_code == 400:
                raise AiloopValidationError(f"Invalid message: {e.response.text}") from e
            else:
                raise ConnectionError(
                    f"HTTP error {e.response.status_code}: {e.response.text}"
                ) from e
        except Exception as e:
            raise ConnectionError(f"Failed to send message: {e}") from e
