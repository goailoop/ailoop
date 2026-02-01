"""Task-related tests for Python SDK."""

from datetime import datetime, timezone
from unittest.mock import AsyncMock, Mock

import pytest

from ailoop import AiloopClient, Task, TaskState, DependencyType
from ailoop.exceptions import ConnectionError, ValidationError


def _task_dict(
    id: str = "task-1",
    title: str = "Test Task",
    description: str = "Test description",
    state: str = "pending",
    metadata: dict | None = None,
    depends_on: list | None = None,
    blocked: bool = False,
) -> dict:
    now = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    return {
        "id": id,
        "title": title,
        "description": description,
        "state": state,
        "created_at": now,
        "updated_at": now,
        "assignee": None,
        "metadata": metadata or {},
        "depends_on": depends_on or [],
        "blocking_for": [],
        "blocked": blocked,
        "dependency_type": None,
    }


@pytest.fixture
async def client():
    """Client with mocked HTTP so no real connection is needed."""
    c = AiloopClient("http://localhost:8080")
    c._http_client = AsyncMock()
    yield c
    if c._http_client:
        await c._http_client.aclose()


@pytest.mark.asyncio
async def test_create_task(client):
    """Test creating a new task."""
    task_data = _task_dict(title="Test Task", description="Test description")
    mock_resp = Mock()
    mock_resp.json.return_value = task_data
    mock_resp.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_resp)

    task = await client.create_task(title="Test Task", description="Test description")

    assert isinstance(task, Task)
    assert task.title == "Test Task"
    assert task.description == "Test description"
    assert task.state == TaskState.PENDING
    assert task.id is not None


@pytest.mark.asyncio
async def test_create_task_with_metadata(client):
    """Test creating a task with metadata."""
    metadata = {"priority": "high", "due_date": "2024-01-31"}
    task_data = _task_dict(metadata=metadata)
    mock_resp = Mock()
    mock_resp.json.return_value = task_data
    mock_resp.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_resp)

    task = await client.create_task(
        title="Test Task", description="Test description", metadata=metadata
    )

    assert task.metadata == metadata


@pytest.mark.asyncio
async def test_update_task_state(client):
    """Test updating task state."""
    task_data = _task_dict(id="t1", title="Test Task", description="Test description")
    updated_data = _task_dict(id="t1", title="Test Task", description="Test description", state="done")
    mock_post = Mock()
    mock_post.json.return_value = task_data
    mock_post.raise_for_status = Mock()
    mock_put = Mock()
    mock_put.json.return_value = updated_data
    mock_put.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.put = AsyncMock(return_value=mock_put)

    created = await client.create_task(title="Test Task", description="Test description")
    updated_task = await client.update_task(task_id=created.id, state="done")

    assert updated_task.state == TaskState.DONE


@pytest.mark.asyncio
async def test_list_tasks(client):
    """Test listing tasks."""
    task_list = [
        _task_dict(id="t1", title="Task 1", description="Description 1"),
        _task_dict(id="t2", title="Task 2", description="Description 2"),
    ]
    mock_resp = Mock()
    mock_resp.json.return_value = {"tasks": task_list}
    mock_resp.raise_for_status = Mock()
    client._http_client.get = AsyncMock(return_value=mock_resp)

    tasks = await client.list_tasks()

    assert len(tasks) >= 2
    assert all(isinstance(t, Task) for t in tasks)


@pytest.mark.asyncio
async def test_list_tasks_by_state(client):
    """Test listing tasks filtered by state."""
    task_list = [_task_dict(id="t1", title="Pending Task", description="Should be pending", state="pending")]
    mock_resp = Mock()
    mock_resp.json.return_value = {"tasks": task_list}
    mock_resp.raise_for_status = Mock()
    client._http_client.get = AsyncMock(return_value=mock_resp)

    pending_tasks = await client.list_tasks(state="pending")

    assert all(t.state == TaskState.PENDING for t in pending_tasks)


@pytest.mark.asyncio
async def test_get_task(client):
    """Test getting a task by ID."""
    task_data = _task_dict(id="t1", title="Test Task", description="Test description")
    mock_resp = Mock()
    mock_resp.json.return_value = task_data
    mock_resp.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_resp)
    client._http_client.get = AsyncMock(return_value=mock_resp)

    created_task = await client.create_task(title="Test Task", description="Test description")
    task = await client.get_task(created_task.id)

    assert task.id == created_task.id
    assert task.title == created_task.title


@pytest.mark.asyncio
async def test_add_dependency(client):
    """Test adding a dependency between tasks."""
    parent_data = _task_dict(id="parent-1", title="Parent Task", description="Parent description")
    child_data = _task_dict(id="child-1", title="Child Task", description="Child description", depends_on=[])
    child_with_dep = _task_dict(id="child-1", title="Child Task", description="Child description", depends_on=["parent-1"])
    mock_post = Mock()
    mock_post.json.side_effect = [parent_data, child_data]
    mock_post.raise_for_status = Mock()
    mock_get = Mock()
    mock_get.json.return_value = child_with_dep
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)

    parent = await client.create_task(title="Parent Task", description="Parent description")
    child = await client.create_task(title="Child Task", description="Child description")
    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    task = await client.get_task(child.id)
    assert len(task.depends_on) == 1
    assert parent.id in task.depends_on


@pytest.mark.asyncio
async def test_add_dependency_with_type(client):
    """Test adding dependency with specific type."""
    parent_data = _task_dict(id="p1", title="Parent Task", description="Parent description")
    child_data = _task_dict(id="c1", title="Child Task", description="Child description")
    child_with_dep = _task_dict(id="c1", title="Child Task", description="Child description", depends_on=["p1"])
    mock_post = Mock()
    mock_post.json.side_effect = [parent_data, child_data]
    mock_post.raise_for_status = Mock()
    mock_get = Mock()
    mock_get.json.return_value = child_with_dep
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)

    parent = await client.create_task(title="Parent Task", description="Parent description")
    child = await client.create_task(title="Child Task", description="Child description")
    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="related")

    task = await client.get_task(child.id)
    assert len(task.depends_on) == 1


@pytest.mark.asyncio
async def test_remove_dependency(client):
    """Test removing a dependency."""
    parent_data = _task_dict(id="p1", title="Parent Task", description="Parent description")
    child_data = _task_dict(id="c1", title="Child Task", description="Child description")
    child_with_dep = _task_dict(id="c1", title="Child Task", description="Child description", depends_on=["p1"])
    child_no_dep = _task_dict(id="c1", title="Child Task", description="Child description", depends_on=[])
    mock_post = Mock()
    mock_post.json.side_effect = [parent_data, child_data, child_with_dep]
    mock_post.raise_for_status = Mock()
    mock_get = Mock()
    mock_get.json.return_value = child_no_dep
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)
    client._http_client.delete = AsyncMock(return_value=Mock(raise_for_status=Mock()))

    parent = await client.create_task(title="Parent Task", description="Parent description")
    child = await client.create_task(title="Child Task", description="Child description")
    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")
    await client.remove_dependency(task_id=child.id, depends_on=parent.id)

    task = await client.get_task(child.id)
    assert len(task.depends_on) == 0


@pytest.mark.asyncio
async def test_get_ready_tasks(client):
    """Test getting tasks that are ready (no blockers)."""
    ready_data = _task_dict(id="r1", title="Ready Task", description="Should be ready")
    mock_post = Mock()
    mock_post.json.return_value = ready_data
    mock_post.raise_for_status = Mock()
    mock_get = Mock()
    mock_get.json.return_value = {"tasks": [ready_data]}
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)

    ready_task = await client.create_task(title="Ready Task", description="Should be ready")
    ready_tasks = await client.get_ready_tasks()

    assert len(ready_tasks) >= 1
    assert ready_task.id in [t.id for t in ready_tasks]


@pytest.mark.asyncio
async def test_get_blocked_tasks(client):
    """Test getting blocked tasks."""
    blocked_data = _task_dict(id="c1", title="Child Task", description="Child description", blocked=True)
    mock_post = Mock()
    mock_post.json.side_effect = [
        _task_dict(id="p1", title="Parent Task", description="Parent description"),
        _task_dict(id="c1", title="Child Task", description="Child description"),
    ]
    mock_post.raise_for_status = Mock()
    mock_get = Mock()
    mock_get.json.return_value = {"tasks": [blocked_data]}
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)

    parent = await client.create_task(title="Parent Task", description="Parent description")
    child = await client.create_task(title="Child Task", description="Child description")
    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    blocked_tasks = await client.get_blocked_tasks()
    assert len(blocked_tasks) >= 1
    assert child.id in [t.id for t in blocked_tasks]


@pytest.mark.asyncio
async def test_get_dependency_graph(client):
    """Test getting dependency graph for a task."""
    parent_data = _task_dict(id="p1", title="Parent Task", description="Parent description")
    child_data = _task_dict(id="c1", title="Child Task", description="Child description")
    mock_post = Mock()
    mock_post.json.side_effect = [parent_data, child_data]
    mock_post.raise_for_status = Mock()
    graph_data = {
        "task": _task_dict(id="c1", title="Child Task", description="Child description"),
        "parents": [parent_data],
        "children": [],
    }
    mock_get = Mock()
    mock_get.json.return_value = graph_data
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)

    parent = await client.create_task(title="Parent Task", description="Parent description")
    child = await client.create_task(title="Child Task", description="Child description")
    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    graph = await client.get_dependency_graph(child.id)
    assert graph["task"]["id"] == child.id
    assert len(graph["parents"]) >= 1
    assert len(graph["children"]) == 0


@pytest.mark.asyncio
async def test_dependency_types(client):
    """Test different dependency types."""
    parent_data = _task_dict(id="p1", title="Parent Task", description="Parent description")
    child_data = _task_dict(id="c1", title="Child Task", description="Child description")
    child_with_dep = _task_dict(id="c1", title="Child Task", description="Child description", depends_on=["p1"])
    mock_post = Mock()
    mock_post.json.side_effect = [parent_data, child_data] + [child_with_dep] * 3
    mock_post.raise_for_status = Mock()
    mock_get = Mock()
    mock_get.json.return_value = child_with_dep
    mock_get.raise_for_status = Mock()
    client._http_client.post = AsyncMock(return_value=mock_post)
    client._http_client.get = AsyncMock(return_value=mock_get)

    parent = await client.create_task(title="Parent Task", description="Parent description")
    child = await client.create_task(title="Child Task", description="Child description")

    for dep_type in ["blocks", "related", "parent"]:
        await client.add_dependency(task_id=child.id, depends_on=parent.id, type=dep_type)
        task = await client.get_task(child.id)
        assert len(task.depends_on) == 1
