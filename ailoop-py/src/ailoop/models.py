"""Data models for ailoop messages."""

from datetime import datetime
from enum import Enum
from typing import Any, Dict, Literal, Optional, Union
from uuid import UUID

from pydantic import BaseModel, Field


class SenderType(str, Enum):
    """Type of message sender."""

    AGENT = "AGENT"
    HUMAN = "HUMAN"


class NotificationPriority(str, Enum):
    """Priority levels for notifications."""

    URGENT = "urgent"
    HIGH = "high"
    NORMAL = "normal"
    LOW = "low"


class ResponseType(str, Enum):
    """Types of responses to questions/authorizations."""

    TEXT = "text"
    AUTHORIZATION_APPROVED = "authorization_approved"
    AUTHORIZATION_DENIED = "authorization_denied"
    TIMEOUT = "timeout"
    CANCELLED = "cancelled"


class QuestionContent(BaseModel):
    """Content for question messages."""

    type: Literal["question"] = "question"
    text: str
    timeout_seconds: int
    choices: Optional[list[str]] = None


class AuthorizationContent(BaseModel):
    """Content for authorization messages."""

    type: Literal["authorization"] = "authorization"
    action: str
    context: Optional[Dict[str, Any]] = None
    timeout_seconds: int


class NotificationContent(BaseModel):
    """Content for notification messages."""

    type: Literal["notification"] = "notification"
    text: str
    priority: NotificationPriority = NotificationPriority.NORMAL


class ResponseContent(BaseModel):
    """Content for response messages."""

    type: Literal["response"] = "response"
    answer: Optional[str] = None
    response_type: ResponseType


class NavigateContent(BaseModel):
    """Content for navigation messages."""

    type: Literal["navigate"] = "navigate"
    url: str


MessageContent = Union[
    QuestionContent,
    AuthorizationContent,
    NotificationContent,
    ResponseContent,
    NavigateContent,
]


class Message(BaseModel):
    """Core message structure."""

    id: UUID
    channel: str
    sender_type: SenderType
    content: MessageContent = Field(discriminator="type")
    timestamp: datetime
    correlation_id: Optional[UUID] = None
    metadata: Optional[Dict[str, Any]] = None

    class Config:
        """Pydantic configuration."""

        use_enum_values = True
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            UUID: str,
        }

    @classmethod
    def create_question(
        cls,
        channel: str,
        text: str,
        timeout_seconds: int = 60,
        choices: Optional[list[str]] = None,
    ) -> "Message":
        """Create a question message."""
        return cls(
            id=UUID(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=QuestionContent(
                text=text,
                timeout_seconds=timeout_seconds,
                choices=choices,
            ),
            timestamp=datetime.utcnow(),
        )

    @classmethod
    def create_authorization(
        cls,
        channel: str,
        action: str,
        timeout_seconds: int = 300,
        context: Optional[Dict[str, Any]] = None,
    ) -> "Message":
        """Create an authorization message."""
        return cls(
            id=UUID(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=AuthorizationContent(
                action=action,
                context=context,
                timeout_seconds=timeout_seconds,
            ),
            timestamp=datetime.utcnow(),
        )

    @classmethod
    def create_notification(
        cls,
        channel: str,
        text: str,
        priority: NotificationPriority = NotificationPriority.NORMAL,
    ) -> "Message":
        """Create a notification message."""
        return cls(
            id=UUID(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=NotificationContent(
                text=text,
                priority=priority,
            ),
            timestamp=datetime.utcnow(),
        )

    @classmethod
    def create_response(
        cls,
        channel: str,
        correlation_id: UUID,
        answer: Optional[str] = None,
        response_type: ResponseType = ResponseType.TEXT,
    ) -> "Message":
        """Create a response message."""
        return cls(
            id=UUID(),
            channel=channel,
            sender_type=SenderType.HUMAN,
            content=ResponseContent(
                answer=answer,
                response_type=response_type,
            ),
            timestamp=datetime.utcnow(),
            correlation_id=correlation_id,
        )
