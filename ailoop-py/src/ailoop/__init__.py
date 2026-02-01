"""Ailoop Python SDK for server communication.

This package provides a Python client for communicating with ailoop servers
via HTTP and WebSocket protocols.
"""

from .client import AiloopClient
from .exceptions import AiloopError, ConnectionError, TimeoutError, ValidationError
from .models import (
    Message,
    MessageContent,
    NotificationPriority,
    ResponseType,
    SenderType,
    Task,
    TaskState,
    DependencyType,
)

__version__ = "0.2.0"
__all__ = [
    "AiloopClient",
    "Message",
    "MessageContent",
    "SenderType",
    "ResponseType",
    "NotificationPriority",
    "Task",
    "TaskState",
    "DependencyType",
    "AiloopError",
    "ConnectionError",
    "TimeoutError",
    "ValidationError",
]
