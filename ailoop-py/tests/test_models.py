"""Tests for ailoop message models."""

from datetime import datetime
from uuid import UUID

from ailoop.models import (
    AuthorizationContent,
    DecisionContent,
    DecisionOption,
    DecisionRecommendation,
    DependencyType,
    Message,
    NotificationContent,
    NotificationPriority,
    ResponseContent,
    ResponseType,
    SenderType,
    Task,
    TaskCreateContent,
    TaskDependencyAddContent,
    TaskDependencyRemoveContent,
    TaskState,
    TaskUpdateContent,
)


class TestMessageModels:
    """Test message model creation and serialization."""

    def test_create_decision(self):
        """Test creating a decision message."""
        options = [
            DecisionOption(id="blue-green", label="Blue/Green", detail_markdown="Zero-downtime swap"),
            DecisionOption(id="canary", label="Canary (10%)"),
        ]
        recommendation = DecisionRecommendation(option_id="blue-green", rationale_markdown="SLO tight")
        message = Message.create_decision(
            channel="test-channel",
            decision_id="deploy-2026",
            summary="Which deployment strategy?",
            options=options,
            timeout_seconds=300,
            context_markdown="Error rate: **0.3%**",
            recommendation=recommendation,
        )

        assert message.channel == "test-channel"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, DecisionContent)
        assert message.content.decision_id == "deploy-2026"
        assert message.content.summary == "Which deployment strategy?"
        assert len(message.content.options) == 2
        assert message.content.options[0].id == "blue-green"
        assert message.content.options[1].id == "canary"
        assert message.content.recommendation.option_id == "blue-green"
        assert isinstance(message.id, UUID)
        assert isinstance(message.timestamp, datetime)

    def test_decision_serialization(self):
        """Test decision message JSON serialization."""
        options = [
            DecisionOption(id="a", label="Option A"),
            DecisionOption(id="b", label="Option B"),
        ]
        message = Message.create_decision(
            channel="test",
            decision_id="test-dec",
            summary="Test?",
            options=options,
            timeout_seconds=60,
        )

        json_data = message.dict()
        assert json_data["channel"] == "test"
        assert json_data["sender_type"] == "AGENT"
        assert json_data["content"]["type"] == "decision"
        assert json_data["content"]["decision_id"] == "test-dec"
        assert len(json_data["content"]["options"]) == 2

        restored = Message(**json_data)
        assert restored.channel == message.channel
        assert restored.content.decision_id == message.content.decision_id

    def test_create_authorization(self):
        """Test creating an authorization message."""
        message = Message.create_authorization(
            channel="admin",
            action="Deploy to production",
            timeout_seconds=300,
            context={"environment": "prod"},
        )

        assert message.channel == "admin"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, AuthorizationContent)
        assert message.content.action == "Deploy to production"
        assert message.content.timeout_seconds == 300
        assert message.content.context == {"environment": "prod"}

    def test_create_notification(self):
        """Test creating a notification message."""
        message = Message.create_notification(
            channel="general",
            text="System maintenance scheduled",
            priority=NotificationPriority.HIGH,
        )

        assert message.channel == "general"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, NotificationContent)
        assert message.content.text == "System maintenance scheduled"
        assert message.content.priority == NotificationPriority.HIGH

    def test_create_response(self):
        """Test creating a response message."""
        original_id = UUID("550e8400-e29b-41d4-a716-446655440000")
        message = Message.create_response(
            channel="test-channel",
            correlation_id=original_id,
            answer="42",
            response_type=ResponseType.TEXT,
        )

        assert message.channel == "test-channel"
        assert message.sender_type == SenderType.HUMAN
        assert isinstance(message.content, ResponseContent)
        assert message.content.answer == "42"
        assert message.content.response_type == ResponseType.TEXT
        assert message.correlation_id == original_id

    def test_message_serialization(self):
        """Test message JSON serialization via decision."""
        options = [
            DecisionOption(id="a", label="A"),
            DecisionOption(id="b", label="B"),
        ]
        message = Message.create_decision(channel="test", decision_id="d", summary="Hello?", options=options, timeout_seconds=30)

        json_data = message.dict()
        assert json_data["channel"] == "test"
        assert json_data["sender_type"] == "AGENT"
        assert json_data["content"]["type"] == "decision"

        restored = Message(**json_data)
        assert restored.channel == message.channel
        assert restored.content.decision_id == message.content.decision_id

    def test_enum_values(self):
        """Test enum string values."""
        assert SenderType.AGENT.value == "AGENT"
        assert SenderType.HUMAN.value == "HUMAN"
        assert ResponseType.TEXT.value == "text"
        assert NotificationPriority.HIGH.value == "high"

    def test_task_creation(self):
        """Test creating a task."""
        task = Task(
            id="550e8400-e29b-41d4-a716-446655440000",
            title="Test Task",
            description="Test Description",
            state=TaskState.PENDING,
            created_at=datetime.utcnow(),
            updated_at=datetime.utcnow(),
        )

        assert task.title == "Test Task"
        assert task.description == "Test Description"
        assert task.state == TaskState.PENDING
        assert task.blocked == False
        assert task.depends_on == []
        assert task.blocking_for == []

    def test_task_create_message(self):
        """Test creating a task create message."""
        task = Task(
            id="550e8400-e29b-41d4-a716-446655440000",
            title="Test Task",
            description="Test Description",
            state=TaskState.PENDING,
            created_at=datetime.utcnow(),
            updated_at=datetime.utcnow(),
        )

        message = Message.create_task_create(channel="public", task=task)

        assert message.channel == "public"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, TaskCreateContent)
        assert message.content.task.title == "Test Task"

    def test_task_update_message(self):
        """Test creating a task update message."""
        message = Message.create_task_update(
            channel="public",
            task_id="550e8400-e29b-41d4-a716-446655440000",
            state=TaskState.DONE,
        )

        assert message.channel == "public"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, TaskUpdateContent)
        assert message.content.state == TaskState.DONE

    def test_task_dependency_add_message(self):
        """Test creating a task dependency add message."""
        message = Message.create_task_dependency_add(
            channel="public",
            task_id="550e8400-e29b-41d4-a716-446655440000",
            depends_on="660e8400-e29b-41d4-a716-446655440001",
            dependency_type=DependencyType.BLOCKS,
        )

        assert message.channel == "public"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, TaskDependencyAddContent)
        assert message.content.dependency_type == DependencyType.BLOCKS

    def test_task_dependency_remove_message(self):
        """Test creating a task dependency remove message."""
        message = Message.create_task_dependency_remove(
            channel="public",
            task_id="550e8400-e29b-41d4-a716-446655440000",
            depends_on="660e8400-e29b-41d4-a716-446655440001",
        )

        assert message.channel == "public"
        assert message.sender_type == SenderType.AGENT
        assert isinstance(message.content, TaskDependencyRemoveContent)

    def test_task_state_enum(self):
        """Test task state enum."""
        assert TaskState.PENDING.value == "pending"
        assert TaskState.DONE.value == "done"
        assert TaskState.ABANDONED.value == "abandoned"

    def test_dependency_type_enum(self):
        """Test dependency type enum."""
        assert DependencyType.BLOCKS.value == "blocks"
        assert DependencyType.RELATED.value == "related"
        assert DependencyType.PARENT.value == "parent"
