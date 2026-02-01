/** Task-related tests for TypeScript SDK. */

import axios from 'axios';
import { AiloopClient, Task, TaskState, DependencyType } from '../src/client';

jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;

type MockTask = Task & { depends_on?: string[] };
const taskStore: MockTask[] = [];
let idCounter = 0;

function makeTask(overrides: Partial<MockTask> = {}): MockTask {
  const id = `task-${++idCounter}`;
  return {
    id,
    title: '',
    description: '',
    state: 'pending',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    depends_on: [],
    ...overrides,
  };
}

const mockAxiosInstance = {
  get: jest.fn(),
  post: jest.fn(),
  put: jest.fn(),
  delete: jest.fn(),
};
mockedAxios.create.mockReturnValue(mockAxiosInstance as any);

describe('Task Operations', () => {
  let client: AiloopClient;

  beforeEach(() => {
    jest.clearAllMocks();
    taskStore.length = 0;
    idCounter = 0;

    mockAxiosInstance.get.mockImplementation((url: string) => {
      if (url.includes('/graph')) {
        const taskId = url.split('/')[4];
        const task = taskStore.find(t => t.id === taskId);
        if (!task) return Promise.reject({ response: { status: 404 } });
        const parents = task.depends_on?.map(pid => taskStore.find(t => t.id === pid)).filter(Boolean) || [];
        return Promise.resolve({ data: { task, parents, children: [] } });
      }
      if (url.includes('/ready')) {
        return Promise.resolve({ data: { tasks: taskStore.filter(t => !t.depends_on?.length) } });
      }
      if (url.includes('/blocked')) {
        return Promise.resolve({ data: { tasks: taskStore.filter(t => (t.depends_on?.length ?? 0) > 0) } });
      }
      if (url.match(/\/api\/v1\/tasks\/[^/]+$/) && !url.includes('?')) {
        const taskId = url.split('/').pop();
        const task = taskStore.find(t => t.id === taskId);
        if (!task) return Promise.reject({ response: { status: 404 } });
        return Promise.resolve({ data: task });
      }
      return Promise.resolve({ data: { tasks: taskStore } });
    });

    mockAxiosInstance.post.mockImplementation((url: string, data: any) => {
      if (url.includes('/dependencies')) {
        const taskId = url.split('/')[4];
        const task = taskStore.find(t => t.id === taskId);
        if (task) {
          const parentId = data?.parent_id;
          if (parentId && !task.depends_on?.includes(parentId)) {
            task.depends_on = [...(task.depends_on || []), parentId];
          }
        }
        return Promise.resolve({ data: {} });
      }
      const task = makeTask({
        title: data?.title,
        description: data?.description,
        state: 'pending',
        metadata: data?.metadata,
      });
      taskStore.push(task);
      return Promise.resolve({ data: task });
    });

    mockAxiosInstance.put.mockImplementation((url: string, data: any) => {
      const taskId = url.split('/')[4];
      const task = taskStore.find(t => t.id === taskId);
      if (!task) return Promise.reject({ response: { status: 404 } });
      task.state = data?.state ?? task.state;
      task.updated_at = new Date().toISOString();
      return Promise.resolve({ data: task });
    });

    mockAxiosInstance.delete.mockImplementation((url: string) => {
      const parts = url.split('/');
      const taskId = parts[4];
      const parentId = parts[6];
      const task = taskStore.find(t => t.id === taskId);
      if (task && task.depends_on) {
        task.depends_on = task.depends_on.filter(id => id !== parentId);
      }
      return Promise.resolve({ data: {} });
    });

    client = new AiloopClient({ baseURL: 'http://localhost:8080' });
    (client as any).httpClient = mockAxiosInstance;
  });

  afterEach(async () => {
    if (client) {
      await client.disconnect().catch(() => {});
    }
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
        expect(task).toHaveProperty('id');
        expect(task).toHaveProperty('title');
        expect(task).toHaveProperty('state');
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
