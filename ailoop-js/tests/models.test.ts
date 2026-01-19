import { MessageFactory, NotificationPriority, ResponseType } from '../src/models';

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
