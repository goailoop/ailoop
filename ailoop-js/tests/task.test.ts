/** Task-related tests for TypeScript SDK. */

import { AiloopClient, Task, TaskState, DependencyType } from './client';

describe('Task Operations', () => {
  let client: AiloopClient;

  beforeEach(() => {
    client = new AiloopClient({ baseURL: 'http://localhost:8080' });
  });

  describe('createTask', () => {
    it('should create a new task', async () => {
      const task = await client.createTask('public', 'Test Task', 'Test description');

      expect(task).toBeDefined();
      expect(task.title).toBe('Test Task');
      expect(task.description).toBe('Test description');
      expect(task.state).toBe('pending');
      expect(task.id).toBeDefined();
    });

    it('should create a task with metadata', async () => {
      const metadata = { priority: 'high', due_date: '2024-01-31' };
      const task = await client.createTask('public', 'Test Task', 'Test description', undefined, metadata);

      expect(task.metadata).toEqual(metadata);
    });
  });

  describe('updateTask', () => {
    it('should update task state to done', async () => {
      const task = await client.createTask('public', 'Test Task', 'Test description');

      const updatedTask = await client.updateTask('public', task.id, 'done');

      expect(updatedTask.state).toBe('done');
    });
  });

  describe('listTasks', () => {
    it('should list tasks', async () => {
      await client.createTask('public', 'Task 1', 'Description 1');
      await client.createTask('public', 'Task 2', 'Description 2');

      const tasks = await client.listTasks('public');

      expect(tasks.length).toBeGreaterThanOrEqual(2);
      tasks.forEach(task => {
        expect(task).toBeInstanceOf(Task);
      });
    });

    it('should filter tasks by state', async () => {
      await client.createTask('public', 'Pending Task', 'Should be pending');

      const pendingTasks = await client.listTasks('public', 'pending');

      pendingTasks.forEach(task => {
        expect(task.state).toBe('pending');
      });
    });
  });

  describe('getTask', () => {
    it('should get a task by ID', async () => {
      const createdTask = await client.createTask('public', 'Test Task', 'Test description');

      const task = await client.getTask(createdTask.id);

      expect(task.id).toBe(createdTask.id);
      expect(task.title).toBe(createdTask.title);
    });
  });

  describe('addDependency', () => {
    it('should add a blocks dependency', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      await client.addDependency(child.id, parent.id, 'blocks');

      const task = await client.getTask(child.id);
      expect(task.depends_on).toContain(parent.id);
    });

    it('should add a related dependency', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      await client.addDependency(child.id, parent.id, 'related');

      const task = await client.getTask(child.id);
      expect(task.depends_on).toContain(parent.id);
    });

    it('should add a parent dependency', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      await client.addDependency(child.id, parent.id, 'parent');

      const task = await client.getTask(child.id);
      expect(task.depends_on).toContain(parent.id);
    });
  });

  describe('removeDependency', () => {
    it('should remove a dependency', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      await client.addDependency(child.id, parent.id, 'blocks');

      await client.removeDependency(child.id, parent.id);

      const task = await client.getTask(child.id);
      expect(task.depends_on).not.toContain(parent.id);
    });
  });

  describe('getReadyTasks', () => {
    it('should return tasks that are ready to start', async () => {
      const readyTask = await client.createTask('public', 'Ready Task', 'Should be ready');

      const blockedTask = await client.createTask('public', 'Blocked Task', 'Should be blocked');

      const readyTasks = await client.getReadyTasks('public');

      expect(readyTasks.length).toBeGreaterThanOrEqual(1);
      const hasReadyTask = readyTasks.some(task => task.id === readyTask.id);
      expect(hasReadyTask).toBe(true);
    });
  });

  describe('getBlockedTasks', () => {
    it('should return blocked tasks', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      await client.addDependency(child.id, parent.id, 'blocks');

      const blockedTasks = await client.getBlockedTasks('public');

      expect(blockedTasks.length).toBeGreaterThanOrEqual(1);
      const hasBlockedTask = blockedTasks.some(task => task.id === child.id);
      expect(hasBlockedTask).toBe(true);
    });
  });

  describe('getDependencyGraph', () => {
    it('should return dependency graph for a task', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      await client.addDependency(child.id, parent.id, 'blocks');

      const graph = await client.getDependencyGraph(child.id);

      expect(graph.task.id).toBe(child.id);
      expect(graph.parents.length).toBeGreaterThanOrEqual(1);
      expect(graph.children.length).toBe(0);
    });
  });

  describe('dependency types', () => {
    it('should support all dependency types', async () => {
      const parent = await client.createTask('public', 'Parent Task', 'Parent description');

      const child = await client.createTask('public', 'Child Task', 'Child description');

      for (const type of ['blocks', 'related', 'parent'] as DependencyType[]) {
        await client.addDependency(child.id, parent.id, type);

        const task = await client.getTask(child.id);
        expect(task.depends_on.length).toBe(1);
      }
    });
  });
});
