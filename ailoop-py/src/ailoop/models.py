"""Data models for ailoop messages."""

from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Literal, Optional, Union
import uuid
from uuid import UUID

from pydantic import BaseModel, Field, ConfigDict


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


class TaskState(str, Enum):
    """Task state."""

    PENDING = "pending"
    DONE = "done"
    ABANDONED = "abandoned"


class DependencyType(str, Enum):
    """Dependency type between tasks."""

    BLOCKS = "blocks"
    RELATED = "related"
    PARENT = "parent"


class TaskCreateContent(BaseModel):
    """Content for task creation messages."""

    type: Literal["task_create"] = "task_create"
    task: "Task"


class TaskUpdateContent(BaseModel):
    """Content for task update messages."""

    type: Literal["task_update"] = "task_update"
    task_id: str
    state: TaskState
    updated_at: datetime


class TaskDependencyAddContent(BaseModel):
    """Content for adding task dependency messages."""

    type: Literal["task_dependency_add"] = "task_dependency_add"
    task_id: str
    depends_on: str
    dependency_type: DependencyType
    timestamp: datetime


class TaskDependencyRemoveContent(BaseModel):
    """Content for removing task dependency messages."""

    type: Literal["task_dependency_remove"] = "task_dependency_remove"
    task_id: str
    depends_on: str
    timestamp: datetime


class Task(BaseModel):
    """Task representation."""

    id: str
    title: str
    description: str
    state: TaskState
    created_at: datetime
    updated_at: datetime
    assignee: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None
    depends_on: List[str] = []
    blocking_for: List[str] = []
    blocked: bool = False
    dependency_type: Optional[DependencyType] = None

    model_config = ConfigDict(
        use_enum_values=True,
        json_encoders={
            datetime: lambda v: v.isoformat(),
        },
    )


MessageContent = Union[
    QuestionContent,
    AuthorizationContent,
    NotificationContent,
    ResponseContent,
    NavigateContent,
    TaskCreateContent,
    TaskUpdateContent,
    TaskDependencyAddContent,
    TaskDependencyRemoveContent,
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

    model_config = ConfigDict(
        use_enum_values=True,
        json_encoders={
            datetime: lambda v: v.isoformat(),
            UUID: str,
        },
    )

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
            id=uuid.uuid4(),
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
            id=uuid.uuid4(),
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
            id=uuid.uuid4(),
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
            id=uuid.uuid4(),
            channel=channel,
            sender_type=SenderType.HUMAN,
            content=ResponseContent(
                answer=answer,
                response_type=response_type,
            ),
            timestamp=datetime.utcnow(),
            correlation_id=correlation_id,
        )

    @classmethod
    def create_task_create(
        cls,
        channel: str,
        task: Task,
    ) -> "Message":
        """Create a task creation message."""
        return cls(
            id=uuid.uuid4(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=TaskCreateContent(task=task),
            timestamp=datetime.utcnow(),
        )

    @classmethod
    def create_task_update(
        cls,
        channel: str,
        task_id: str,
        state: TaskState,
    ) -> "Message":
        """Create a task update message."""
        return cls(
            id=uuid.uuid4(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=TaskUpdateContent(
                task_id=task_id,
                state=state,
                updated_at=datetime.utcnow(),
            ),
            timestamp=datetime.utcnow(),
        )

    @classmethod
    def create_task_dependency_add(
        cls,
        channel: str,
        task_id: str,
        depends_on: str,
        dependency_type: DependencyType,
    ) -> "Message":
        """Create a task dependency addition message."""
        return cls(
            id=uuid.uuid4(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=TaskDependencyAddContent(
                task_id=task_id,
                depends_on=depends_on,
                dependency_type=dependency_type,
                timestamp=datetime.utcnow(),
            ),
            timestamp=datetime.utcnow(),
        )

    @classmethod
    def create_task_dependency_remove(
        cls,
        channel: str,
        task_id: str,
        depends_on: str,
    ) -> "Message":
        """Create a task dependency removal message."""
        return cls(
            id=uuid.uuid4(),
            channel=channel,
            sender_type=SenderType.AGENT,
            content=TaskDependencyRemoveContent(
                task_id=task_id,
                depends_on=depends_on,
                timestamp=datetime.utcnow(),
            ),
            timestamp=datetime.utcnow(),
        )
