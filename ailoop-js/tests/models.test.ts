import { MessageFactory, NotificationPriority, ResponseType, Task, TaskState, DependencyType } from '../src/models';

describe('MessageFactory', () => {
  describe('createQuestion', () => {
    it('should create a question message with minimal options', () => {
      const message = MessageFactory.createQuestion('test-channel', 'What is the answer?');

      expect(message.channel).toBe('test-channel');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('question');
      expect(message.content.text).toBe('What is the answer?');
      expect(message.content.timeout_seconds).toBe(60);
      expect(message.content.choices).toBeUndefined();
    });

    it('should create a question message with all options', () => {
      const message = MessageFactory.createQuestion(
        'test-channel',
        'What is the answer?',
        120,
        ['A', 'B', 'C']
      );

      expect(message.content.timeout_seconds).toBe(120);
      expect(message.content.choices).toEqual(['A', 'B', 'C']);
    });
  });

  describe('createAuthorization', () => {
    it('should create an authorization message', () => {
      const message = MessageFactory.createAuthorization(
        'admin-channel',
        'Deploy to production',
        300,
        { environment: 'prod' }
      );

      expect(message.channel).toBe('admin-channel');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('authorization');
      expect(message.content.action).toBe('Deploy to production');
      expect(message.content.timeout_seconds).toBe(300);
      expect(message.content.context).toEqual({ environment: 'prod' });
    });
  });

  describe('createNotification', () => {
    it('should create a notification message', () => {
      const message = MessageFactory.createNotification(
        'general',
        'System maintenance',
        'high' as any
      );

      expect(message.channel).toBe('general');
      expect(message.content.type).toBe('notification');
      expect(message.content.text).toBe('System maintenance');
      expect(message.content.priority).toBe('high');
    });
  });

  describe('createResponse', () => {
    it('should create a response message', () => {
      const message = MessageFactory.createResponse(
        'msg-123',
        'Yes',
        'text' as any
      );

      expect(message.sender_type).toBe('HUMAN');
      expect(message.correlation_id).toBe('msg-123');
      expect(message.content.type).toBe('response');
      expect(message.content.answer).toBe('Yes');
      expect(message.content.response_type).toBe('text' as any);
    });
  });

  describe('createNavigate', () => {
    it('should create a navigation message', () => {
      const message = MessageFactory.createNavigate('ui-channel', 'https://example.com');

      expect(message.channel).toBe('ui-channel');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('navigate');
      expect(message.content.url).toBe('https://example.com');
    });
  });

  describe('Task Messages', () => {
    it('should create a task create message', () => {
      const task: Task = {
        id: 'task-123',
        title: 'Test Task',
        description: 'Test Description',
        state: 'pending',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        depends_on: [],
        blocking_for: [],
        blocked: false,
      };

      const message = MessageFactory.createTaskCreate('public', task);

      expect(message.channel).toBe('public');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('task_create');
      expect(message.content.task.title).toBe('Test Task');
    });

    it('should create a task update message', () => {
      const message = MessageFactory.createTaskUpdate('public', 'task-123', 'done');

      expect(message.channel).toBe('public');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('task_update');
      expect(message.content.task_id).toBe('task-123');
      expect(message.content.state).toBe('done');
    });

    it('should create a task dependency add message', () => {
      const message = MessageFactory.createTaskDependencyAdd(
        'public',
        'child-task-123',
        'parent-task-456',
        'blocks'
      );

      expect(message.channel).toBe('public');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('task_dependency_add');
      expect(message.content.task_id).toBe('child-task-123');
      expect(message.content.depends_on).toBe('parent-task-456');
      expect(message.content.dependency_type).toBe('blocks');
    });

    it('should create a task dependency remove message', () => {
      const message = MessageFactory.createTaskDependencyRemove(
        'public',
        'child-task-123',
        'parent-task-456'
      );

      expect(message.channel).toBe('public');
      expect(message.sender_type).toBe('AGENT');
      expect(message.content.type).toBe('task_dependency_remove');
      expect(message.content.task_id).toBe('child-task-123');
      expect(message.content.depends_on).toBe('parent-task-456');
    });
  });
});

  describe('Types', () => {
    it('should export correct type values', () => {
      // Test that the types are correctly defined
      const priority: 'low' | 'normal' | 'high' | 'urgent' = 'high';
      const responseType: 'text' | 'authorization_approved' | 'authorization_denied' | 'timeout' | 'cancelled' = 'text';

      expect(priority).toBe('high');
      expect(responseType).toBe('text');
    });
  });
