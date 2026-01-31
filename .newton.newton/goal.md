# F001: Task Command Feature

## Overview
Add task management functionality to ailoop (ailoop-cli, ailoop-core and SDKs) allowing AI agents to create, track, and manage tasks as structured work items that require human oversight or completion.

## Core Concept
Tasks represent discrete units of work that AI agents can create and humans can track through completion states. This enables AI agents to break down complex workflows into manageable, trackable items while maintaining human oversight of progress.

## Requirements

### Task States
- **pending**: Task created, awaiting action
- **done**: Task completed successfully
- **abandoned**: Task cancelled or no longer needed

### Task Structure
Each task should include:
- **id**: Unique identifier (UUID)
- **title**: Brief descriptive title
- **description**: Detailed task explanation
- **state**: Current status (pending/done/abandoned)
- **created_at**: Timestamp when task was created
- **updated_at**: Timestamp of last state change
- **assignee**: Optional human assignee identifier
- **metadata**: Optional structured data for task-specific information
- **depends_on**: List of parent task IDs (dependencies)
- **blocking_for**: List of child task IDs (computed, tasks blocked by this task)
- **blocked**: Boolean flag indicating if task is blocked by dependencies
- **dependency_type**: Type of relationship ("blocks", "related", "parent")

### CLI Commands

#### Create Task
```bash
ailoop task create "Task Title" --description "Detailed description" --channel public
```

#### List Tasks
```bash
ailoop task list [--state pending|done|abandoned] [--channel public] [--json]
```

#### Update Task State
```bash
ailoop task update <task-id> --state done|abandoned [--channel public]
```

#### Get Task Details
```bash
ailoop task show <task-id> [--channel public] [--json]
```

#### Dependency Management
```bash
# Add dependency relationship
ailoop task dep add <child-task-id> <parent-task-id> [--type blocks|related|parent] [--channel public]

# Remove dependency
ailoop task dep remove <child-task-id> <parent-task-id> [--channel public]

# List ready tasks (no open blockers)
ailoop task ready [--channel public] [--json]

# List blocked tasks
ailoop task blocked [--channel public] [--json]

# Show dependency graph
ailoop task dep graph <task-id> [--channel public]
```

### Message Types

#### Task Creation Message
```json
{
  "type": "task",
  "action": "create",
  "task": {
    "id": "uuid",
    "title": "Task Title",
    "description": "Detailed description",
    "state": "pending",
    "created_at": "2024-01-20T10:00:00Z",
    "updated_at": "2024-01-20T10:00:00Z"
  }
}
```

#### Task Update Message
```json
{
  "type": "task",
  "action": "update",
  "task_id": "uuid",
  "state": "done",
  "updated_at": "2024-01-20T11:00:00Z"
}
```

#### Dependency Addition Message
```json
{
  "type": "task",
  "action": "dependency_add",
  "task_id": "child-uuid",
  "depends_on": "parent-uuid",
  "dependency_type": "blocks",
  "timestamp": "2024-01-20T12:00:00Z"
}
```

#### Dependency Removal Message
```json
{
  "type": "task",
  "action": "dependency_remove",
  "task_id": "child-uuid",
  "depends_on": "parent-uuid",
  "timestamp": "2024-01-20T12:01:00Z"
}
```

### SDK Integration

#### Python SDK
```python
from ailoop import AiloopClient

client = AiloopClient()

# Create task
task = await client.create_task(
    channel="public",
    title="Review code changes",
    description="Review PR #123 for security issues"
)

# Update task
await client.update_task(
    channel="public",
    task_id=task.id,
    state="done"
)

# List tasks
tasks = await client.list_tasks(channel="public", state="pending")

# Add dependency
await client.add_dependency(
    channel="public",
    task_id="child-id",
    depends_on="parent-id",
    type="blocks"
)

# Get ready tasks (no blockers)
ready_tasks = await client.get_ready_tasks(channel="public")

# Get dependency graph
graph = await client.get_dependency_graph(channel="public", task_id="id")
```

#### TypeScript SDK
```typescript
import { AiloopClient } from 'ailoop-js';

const client = new AiloopClient();

// Create task
const task = await client.createTask('public', {
  title: 'Review code changes',
  description: 'Review PR #123 for security issues'
});

// Update task
await client.updateTask('public', task.id, 'done');

// List tasks
const tasks = await client.listTasks('public', 'pending');

// Add dependency
await client.addDependency('public', 'child-id', 'parent-id', 'blocks');

// Get ready tasks (no blockers)
const readyTasks = await client.getReadyTasks('public');

// Get dependency graph
const graph = await client.getDependencyGraph('public', 'id');
```

### Server Storage
Tasks should be persisted server-side with:
- In-memory storage for development
- Optional database backend for production
- Channel-based isolation
- Task history and state transitions

### API Endpoints
- `POST /api/v1/tasks` - Create task
- `GET /api/v1/tasks` - List tasks (with filtering)
- `GET /api/v1/tasks/{id}` - Get task details
- `PUT /api/v1/tasks/{id}` - Update task
- `DELETE /api/v1/tasks/{id}` - Delete task (soft delete)
- `POST /api/v1/tasks/{id}/dependencies` - Add dependency
- `DELETE /api/v1/tasks/{id}/dependencies/{dep_id}` - Remove dependency
- `GET /api/v1/tasks/ready` - Get tasks with no open blockers
- `GET /api/v1/tasks/blocked` - Get tasks with open dependencies
- `GET /api/v1/tasks/{id}/dependencies` - Get task dependencies
- `GET /api/v1/tasks/{id}/graph` - Get dependency graph visualization

### Web Interface Integration
Tasks should be displayable in the web UI with:
- Task list view with filtering
- Task detail view
- State transition buttons
- Real-time updates via WebSocket

### Implementation Components

#### Core Library (ailoop-core)
- Add `Task` and `TaskMessage` structs to models
- Implement task validation and state management
- Add task storage interfaces

#### CLI (ailoop-cli)
- Add `task` subcommand with create/list/update/show subcommands
- Implement handlers for task operations
- Add JSON output support

#### Python SDK (ailoop-py)
- Add `Task` and `TaskContent` models
- Implement async methods: `create_task`, `update_task`, `list_tasks`, `get_task`
- Add task factory methods to Message class

#### TypeScript SDK (ailoop-js)
- Add `Task` and `TaskContent` interfaces
- Implement methods: `createTask`, `updateTask`, `listTasks`, `getTask`
- Add task factory methods to MessageFactory class

### Backward Compatibility
- Task functionality is additive - existing message types remain unchanged
- Optional feature that can be disabled if not needed
- Graceful degradation when task features aren't supported

### Testing
- Unit tests for task models and validation
- Integration tests for CLI commands
- SDK tests for all language bindings
- End-to-end tests with server mode

### Future Extensions
- Task assignment to specific users
- Task prioritization
- Task templates and categories
- Integration with external task management systems
- Hierarchical task IDs (epic → task → sub-task)

## Quick Start

### Basic Task Workflow

```bash
# 1. Create parent task
TASK1=$(ailoop task create "Setup database" --description "Initialize PostgreSQL database" --channel public --json | jq -r '.id')

# 2. Create child task that depends on parent
TASK2=$(ailoop task create "Create users table" --description "Create users table in database" --channel public --json | jq -r '.id')

# 3. Add dependency: TASK2 blocks until TASK1 is done
ailoop task dep add "$TASK2" "$TASK1" --type blocks --channel public

# 4. Check ready tasks (should only show TASK1)
ailoop task ready --channel public

# 5. Complete parent task
ailoop task update "$TASK1" --state done --channel public

# 6. Check ready tasks again (should now show TASK2)
ailoop task ready --channel public

# 7. Complete child task
ailoop task update "$TASK2" --state done --channel public

# 8. Show dependency graph
ailoop task dep graph "$TASK2" --channel public
```

### Python SDK Quick Start

```python
from ailoop import AiloopClient
import asyncioasync def main():
    client = AiloopClient()

    # Create tasks with dependency
    parent = await client.create_task(
        channel="public",
        title="Setup database",
        description="Initialize PostgreSQL database"
    )

    child = await client.create_task(
        channel="public",
        title="Create users table",
        description="Create users table in database"
    )

    # Add blocking dependency
    await client.add_dependency(
        channel="public",
        task_id=child.id,
        depends_on=parent.id,
        type="blocks"
    )

    # Get ready tasks
    ready = await client.get_ready_tasks(channel="public")
    print(f"Ready tasks: {len(ready)}")  # Should be 1 (parent only)

    # Complete parent
    await client.update_task(channel="public", task_id=parent.id, state="done")

    # Check ready tasks again
    ready = await client.get_ready_tasks(channel="public")
    print(f"Ready tasks: {len(ready)}")  # Should be 1 (child now ready)

asyncio.run(main())
```

### TypeScript SDK Quick Start

```typescript
import { AiloopClient } from 'ailoop-js';

async function main() {
  const client = new AiloopClient();

  // Create tasks with dependency
  const parent = await client.createTask('public', {
    title: 'Setup database',
    description: 'Initialize PostgreSQL database'
  });

  const child = await client.createTask('public', {
    title: 'Create users table',
    description: 'Create users table in database'
  });

  // Add blocking dependency
  await client.addDependency('public', child.id, parent.id, 'blocks');

  // Get ready tasks
  const ready = await client.getReadyTasks('public');
  console.log(`Ready tasks: ${ready.length}`); // Should be 1 (parent only)

  // Complete parent
  await client.updateTask('public', parent.id, 'done');

  // Check ready tasks again
  const ready2 = await client.getReadyTasks('public');
  console.log(`Ready tasks: ${ready2.length}`); // Should be 1 (child now ready)
}

main();
```

## Dependency System

### Dependency Types

- **blocks**: Parent task must be in `done` state before child can start. Child task automatically becomes `blocked=true` when parent is not done.
- **related**: Tasks are associated but not blocking. Useful for grouping related work items without enforcing order.
- **parent**: Hierarchical relationship for epics and sub-tasks. Automatically sets up blocking relationship.

### Blocking Logic

Automatic state transitions:
- When a task depends on a `pending` task → automatically set `blocked=true`
- When parent task moves to `done` → check if all dependencies are done → set child `blocked=false`
- When parent task is `abandoned` → automatically block child (or cascade abandonment based on configuration)

### Dependency Validation

- Prevent circular dependencies (A→B→C→A)
- Validate dependency exists when adding
- Auto-update `blocked` flag on dependency changes
- Cascade abandonment when parent is abandoned

### Performance OptimizationDependency caching using `blocked_tasks_cache` table (similar to Beads' 25x optimization):
```go
type DependencyCache struct {
    ReadyTasks      []string
    BlockedTasks    []string
    LastUpdated     time.Time
}
```

### Database Schema Additions

```sql
CREATE TABLE task_dependencies (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    dependency_type TEXT DEFAULT 'blocks',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (depends_on_id) REFERENCES tasks(id),
    UNIQUE(task_id, depends_on_id)
);

CREATE INDEX idx_task_deps_task ON task_dependencies(task_id);
CREATE INDEX idx_task_deps_parent ON task_dependencies(depends_on_id);

-- Cache for performance
CREATE TABLE blocked_tasks_cache (
    task_id TEXT PRIMARY KEY,
    blocked_count INTEGER DEFAULT 0,
    last_checked TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Test Plan

### Unit Tests

#### Task Model Tests
- Test task creation with all fields
- Test task state transitions (pending → done → abandoned)
- Test dependency addition/removal
- Test circular dependency detection
- Test blocked flag calculation
- Test validation of dependency types

#### Dependency Logic Tests
- Test `blocked` flag is set when parent is pending
- Test `blocked` flag is cleared when parent becomes done
- Test `blocked` flag is set when parent is abandoned
- Test all dependencies must be done for task to be unblocked
- Test dependency type enforcement

#### Cache Tests
- Test dependency cache population
- Test cache invalidation on dependency changes
- Test cache performance for ready/blocked queries

### Integration Tests

#### CLI Commands
- Test `ailoop task create` with and without dependencies
- Test `ailoop task dep add` with all dependency types
- Test `ailoop task dep remove`
- Test `ailoop task ready` filtering
- Test `ailoop task blocked` filtering
- Test `ailoop task dep graph` output
- Test JSON output format for all commands

#### API Endpoints
- Test POST `/api/v1/tasks` with dependencies
- Test POST `/api/v1/tasks/{id}/dependencies`
- Test DELETE `/api/v1/tasks/{id}/dependencies/{dep_id}`
- Test GET `/api/v1/tasks/ready` returns correct tasks
- Test GET `/api/v1/tasks/blocked` returns correct tasks
- Test GET `/api/v1/tasks/{id}/dependencies`
- Test GET `/api/v1/tasks/{id}/graph` returns valid graph

#### Message Protocol
- Test task creation message format
- Test task update message format
- Test dependency add message format
- Test dependency remove message format
- Test message propagation through WebSocket

### SDK Tests

#### Python SDK
- Test `create_task` with and without dependencies
- Test `add_dependency` with all types
- Test `remove_dependency`
- Test `get_ready_tasks` filtering
- Test `get_dependency_graph` output
- Test `update_task` triggers dependency recalculation
- Test error handling for invalid dependencies

#### TypeScript SDK
- Test `createTask` with and without dependencies
- Test `addDependency` with all types
- Test `removeDependency`
- Test `getReadyTasks` filtering
- Test `getDependencyGraph` output
- Test `updateTask` triggers dependency recalculation
- Test error handling for invalid dependencies

### End-to-End Tests

#### Basic Workflow
1. Create parent task
2. Create child task
3. Add blocking dependency
4. Verify child is blocked
5. Complete parent task
6. Verify child becomes unblocked
7. Complete child task
8. Verify both tasks are done

#### Complex Workflow
1. Create task A (parent)
2. Create task B (depends on A)
3. Create task C (depends on A)
4. Create task D (depends on B and C)
5. Add all dependencies
6. Verify only task A is ready
7. Complete task A
8. Verify tasks B and C are ready, D is still blocked
9. Complete task B
10. Verify only task C is ready, D still blocked
11. Complete task C
12. Verify task D becomes ready
13. Complete task D
14. Verify all tasks are done

#### Circular Dependency Prevention
1. Create task A
2. Create task B
3. Create task C
4. Add dependency A → B
5. Add dependency B → C
6. Attempt to add dependency C → A
7. Verify error is returned (circular dependency)

#### Multi-Channel Isolation
1. Create tasks in channel "public"
2. Create tasks in channel "private"
3. Add dependencies within each channel
4. Verify dependencies don't cross channels
5. Verify `task ready` respects channel filtering

#### Performance Tests
- Test ready query performance with 1000 tasks
- Test blocked query performance with 1000 tasks
- Test dependency graph generation with complex hierarchies
- Measure cache effectiveness for repeated queries

#### Concurrency Tests
- Test multiple agents adding dependencies simultaneously
- Test race conditions when parent completes and child becomes ready
- Test cache consistency under concurrent updates

### Test Execution Order

1. **Phase 1: Unit Tests** - Run first, fast feedback
2. **Phase 2: Integration Tests** - CLI and API endpoints
3. **Phase 3: SDK Tests** - Python and TypeScript
4. **Phase 4: End-to-End Tests** - Full workflows
5. **Phase 5: Performance Tests** - Load testing
6. **Phase 6: Concurrency Tests** - Multi-agent scenarios

### Test Coverage Targets

- Code coverage: > 80%
- API endpoint coverage: 100%
- SDK method coverage: 100%
- CLI command coverage: 100%
- Edge case coverage: All dependency types and states

You must ensure you have enough tests that use cargo-insta to do snapshot tests and confirm version number is available.
