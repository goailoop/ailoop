"""Tests for ailoop client."""

from unittest.mock import AsyncMock, Mock, patch

import pytest
from httpx import Response

from ailoop.client import AiloopClient
from ailoop.exceptions import ConnectionError, ValidationError
from ailoop.models import Message, ResponseType


class TestAiloopClient:
    """Test AiloopClient functionality."""

    @pytest.fixture
    async def client(self):
        """Create a test client."""
        client = AiloopClient("http://test-server:8080")
        # Mock the HTTP client to avoid real connections
        client._http_client = AsyncMock()
        yield client
        # Cleanup
        if client._http_client:
            await client._http_client.aclose()

    @pytest.mark.asyncio
    async def test_connect_success(self, client):
        """Test successful connection."""
        # Mock health check response
        mock_response = Mock()
        mock_response.json.return_value = {"status": "healthy", "version": "0.1.1"}
        mock_response.raise_for_status = Mock()

        # Mock httpx.AsyncClient
        mock_http_client = AsyncMock()
        mock_http_client.get = AsyncMock(return_value=mock_response)

        with patch('httpx.AsyncClient', return_value=mock_http_client):
            await client.connect()

            assert client._http_client is not None
            mock_http_client.get.assert_called_once_with("/api/v1/health")

    @pytest.mark.asyncio
    async def test_connect_failure(self, client):
        """Test connection failure."""
        client._http_client.get = AsyncMock(side_effect=Exception("Connection failed"))

        with pytest.raises(ConnectionError, match="Failed to connect"):
            await client.connect()

    @pytest.mark.asyncio
    async def test_say_notification(self, client):
        """Test sending a notification."""
        # Mock successful response
        mock_response = Mock()
        mock_response.json.return_value = {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "channel": "test",
            "sender_type": "AGENT",
            "content": {"type": "notification", "text": "Hello", "priority": "normal"},
            "timestamp": "2024-01-15T12:00:00Z",
        }
        mock_response.raise_for_status = Mock()

        client._http_client.post = AsyncMock(return_value=mock_response)

        result = await client.say("Hello", channel="test")

        assert isinstance(result, Message)
        assert result.content.text == "Hello"
        assert result.channel == "test"

        client._http_client.post.assert_called_once()
        call_args = client._http_client.post.call_args
        assert call_args[0][0] == "/api/v1/messages"

    @pytest.mark.asyncio
    async def test_ask_question(self, client):
        """Test asking a question."""
        # Mock successful response
        mock_response = Mock()
        mock_response.json.return_value = {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "channel": "test",
            "sender_type": "AGENT",
            "content": {
                "type": "question",
                "text": "What is 2+2?",
                "timeout_seconds": 60,
                "choices": ["3", "4", "5"],
            },
            "timestamp": "2024-01-15T12:00:00Z",
        }
        mock_response.raise_for_status = Mock()

        client._http_client.post = AsyncMock(return_value=mock_response)

        result = await client.ask("What is 2+2?", channel="test", choices=["3", "4", "5"])

        assert isinstance(result, Message)
        assert result.content.text == "What is 2+2?"
        assert result.content.choices == ["3", "4", "5"]

    @pytest.mark.asyncio
    async def test_get_message(self, client):
        """Test getting a message by ID."""
        message_id = "550e8400-e29b-41d4-a716-446655440000"

        # Mock successful response
        mock_response = Mock()
        mock_response.json.return_value = {
            "id": message_id,
            "channel": "test",
            "sender_type": "AGENT",
            "content": {"type": "notification", "text": "Test", "priority": "normal"},
            "timestamp": "2024-01-15T12:00:00Z",
        }
        mock_response.raise_for_status = Mock()

        client._http_client.get = AsyncMock(return_value=mock_response)

        result = await client.get_message(message_id)

        assert isinstance(result, Message)
        assert str(result.id) == message_id

        client._http_client.get.assert_called_once_with(f"/api/v1/messages/{message_id}")

    @pytest.mark.asyncio
    async def test_get_message_not_found(self, client):
        """Test getting a non-existent message."""
        message_id = "550e8400-e29b-41d4-a716-446655440999"

        # Mock 404 response
        from httpx import HTTPStatusError

        mock_response = Response(404, json={"error": "Message not found"})
        client._http_client.get = AsyncMock(
            side_effect=HTTPStatusError("Not found", request=Mock(), response=mock_response)
        )

        with pytest.raises(ValidationError, match="Message not found"):
            await client.get_message(message_id)

    @pytest.mark.asyncio
    async def test_respond_to_message(self, client):
        """Test responding to a message."""
        original_id = "550e8400-e29b-41d4-a716-446655440000"

        # Mock get_message response
        get_response = Mock()
        get_response.json.return_value = {
            "id": original_id,
            "channel": "test",
            "sender_type": "AGENT",
            "content": {"type": "question", "text": "Test?", "timeout_seconds": 60},
            "timestamp": "2024-01-15T12:00:00Z",
        }
        get_response.raise_for_status = Mock()

        # Mock post response
        post_response = Mock()
        post_response.json.return_value = {
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "channel": "test",
            "sender_type": "HUMAN",
            "content": {"type": "response", "answer": "Yes", "response_type": "text"},
            "timestamp": "2024-01-15T12:01:00Z",
            "correlation_id": original_id,
        }
        post_response.raise_for_status = Mock()

        client._http_client.get = AsyncMock(return_value=get_response)
        client._http_client.post = AsyncMock(return_value=post_response)

        result = await client.respond(original_id, answer="Yes", response_type=ResponseType.TEXT)

        assert isinstance(result, Message)
        assert result.content.answer == "Yes"
        assert str(result.correlation_id) == original_id

    @pytest.mark.asyncio
    async def test_version_compatibility(self, client):
        """Test version compatibility checking."""
        # Mock health response
        mock_response = Mock()
        mock_response.json.return_value = {
            "status": "healthy",
            "version": "0.1.1",
            "active_connections": 5,
        }
        mock_response.raise_for_status = Mock()

        client._http_client.get = AsyncMock(return_value=mock_response)

        result = await client.check_version_compatibility()

        assert result["server_version"] == "0.1.1"
        assert result["client_version"] == "0.1.1"
        assert result["compatible"] is True
        assert result["health_data"]["active_connections"] == 5
