"""Ailoop Python SDK for server communication.

This package provides a Python client for communicating with ailoop servers
via HTTP and WebSocket protocols.
"""

from .client import AiloopClient
from .exceptions import AiloopError, ConnectionError, ValidationError
from .models import Message, MessageContent, NotificationPriority, ResponseType, SenderType

__version__ = "0.1.1"
__all__ = [
    "AiloopClient",
    "Message",
    "MessageContent",
    "SenderType",
    "ResponseType",
    "NotificationPriority",
    "AiloopError",
    "ConnectionError",
    "ValidationError",
]
