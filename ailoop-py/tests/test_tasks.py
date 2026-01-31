"""Task-related tests for Python SDK."""

import pytest
from ailoop import AiloopClient, Task, TaskState, DependencyType
from ailoop.exceptions import ConnectionError, ValidationError


@pytest.mark.asyncio
async def test_create_task():
    """Test creating a new task."""
    client = AiloopClient("http://localhost:8080")

    task = await client.create_task(title="Test Task", description="Test description")

    assert isinstance(task, Task)
    assert task.title == "Test Task"
    assert task.description == "Test description"
    assert task.state == TaskState.PENDING
    assert task.id is not None


@pytest.mark.asyncio
async def test_create_task_with_metadata():
    """Test creating a task with metadata."""
    client = AiloopClient("http://localhost:8080")

    metadata = {"priority": "high", "due_date": "2024-01-31"}
    task = await client.create_task(
        title="Test Task", description="Test description", metadata=metadata
    )

    assert task.metadata == metadata


@pytest.mark.asyncio
async def test_update_task_state():
    """Test updating task state."""
    client = AiloopClient("http://localhost:8080")

    task = await client.create_task(title="Test Task", description="Test description")

    updated_task = await client.update_task(task_id=task.id, state="done")

    assert updated_task.state == TaskState.DONE


@pytest.mark.asyncio
async def test_list_tasks():
    """Test listing tasks."""
    client = AiloopClient("http://localhost:8080")

    await client.create_task(title="Task 1", description="Description 1")
    await client.create_task(title="Task 2", description="Description 2")

    tasks = await client.list_tasks()

    assert len(tasks) >= 2
    assert all(isinstance(task, Task) for task in tasks)


@pytest.mark.asyncio
async def test_list_tasks_by_state():
    """Test listing tasks filtered by state."""
    client = AiloopClient("http://localhost:8080")

    await client.create_task(title="Pending Task", description="Should be pending")

    pending_tasks = await client.list_tasks(state="pending")

    assert all(task.state == TaskState.PENDING for task in pending_tasks)


@pytest.mark.asyncio
async def test_get_task():
    """Test getting a task by ID."""
    client = AiloopClient("http://localhost:8080")

    created_task = await client.create_task(title="Test Task", description="Test description")

    task = await client.get_task(created_task.id)

    assert task.id == created_task.id
    assert task.title == created_task.title


@pytest.mark.asyncio
async def test_add_dependency():
    """Test adding a dependency between tasks."""
    client = AiloopClient("http://localhost:8080")

    parent = await client.create_task(title="Parent Task", description="Parent description")

    child = await client.create_task(title="Child Task", description="Child description")

    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    task = await client.get_task(child.id)
    assert len(task.depends_on) == 1
    assert parent.id in task.depends_on


@pytest.mark.asyncio
async def test_add_dependency_with_type():
    """Test adding dependency with specific type."""
    client = AiloopClient("http://localhost:8080")

    parent = await client.create_task(title="Parent Task", description="Parent description")

    child = await client.create_task(title="Child Task", description="Child description")

    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="related")

    task = await client.get_task(child.id)
    assert len(task.depends_on) == 1


@pytest.mark.asyncio
async def test_remove_dependency():
    """Test removing a dependency."""
    client = AiloopClient("http://localhost:8080")

    parent = await client.create_task(title="Parent Task", description="Parent description")

    child = await client.create_task(title="Child Task", description="Child description")

    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    await client.remove_dependency(task_id=child.id, depends_on=parent.id)

    task = await client.get_task(child.id)
    assert len(task.depends_on) == 0


@pytest.mark.asyncio
async def test_get_ready_tasks():
    """Test getting tasks that are ready (no blockers)."""
    client = AiloopClient("http://localhost:8080")

    ready_task = await client.create_task(title="Ready Task", description="Should be ready")

    blocked_task = await client.create_task(title="Blocked Task", description="Should be blocked")

    ready_tasks = await client.get_ready_tasks()

    assert len(ready_tasks) >= 1
    assert ready_task.id in [task.id for task in ready_tasks]


@pytest.mark.asyncio
async def test_get_blocked_tasks():
    """Test getting blocked tasks."""
    client = AiloopClient("http://localhost:8080")

    parent = await client.create_task(title="Parent Task", description="Parent description")

    child = await client.create_task(title="Child Task", description="Child description")

    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    blocked_tasks = await client.get_blocked_tasks()

    assert len(blocked_tasks) >= 1
    assert child.id in [task.id for task in blocked_tasks]


@pytest.mark.asyncio
async def test_get_dependency_graph():
    """Test getting dependency graph for a task."""
    client = AiloopClient("http://localhost:8080")

    parent = await client.create_task(title="Parent Task", description="Parent description")

    child = await client.create_task(title="Child Task", description="Child description")

    await client.add_dependency(task_id=child.id, depends_on=parent.id, type="blocks")

    graph = await client.get_dependency_graph(child.id)

    assert graph["task"]["id"] == child.id
    assert len(graph["parents"]) >= 1
    assert len(graph["children"]) == 0


@pytest.mark.asyncio
async def test_dependency_types():
    """Test different dependency types."""
    client = AiloopClient("http://localhost:8080")

    parent = await client.create_task(title="Parent Task", description="Parent description")

    child = await client.create_task(title="Child Task", description="Child description")

    for dep_type in ["blocks", "related", "parent"]:
        await client.add_dependency(task_id=child.id, depends_on=parent.id, type=dep_type)

        task = await client.get_task(child.id)
        assert len(task.depends_on) == 1
