import axios from 'axios';
import { AiloopClient } from '../src/client';
import { NotificationPriority } from '../src/models';

jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;
const mockAxiosInstance = {
  get: jest.fn(),
  post: jest.fn(),
};
mockedAxios.create.mockReturnValue(mockAxiosInstance as any);

describe('AiloopClient HTTP Methods', () => {
  let client: AiloopClient;

  beforeEach(() => {
    jest.clearAllMocks();
    mockAxiosInstance.get.mockClear();
    mockAxiosInstance.post.mockClear();

    client = new AiloopClient({ baseURL: 'http://localhost:8080' });
    // Override the client's httpClient with our mock
    (client as any).httpClient = mockAxiosInstance;
  });

  describe('checkHealth', () => {
    it('should return health response', async () => {
      const mockResponse = {
        status: 'healthy',
        version: '0.1.1',
        activeConnections: 5,
        queueSize: 10,
        activeChannels: 3,
      };

      mockAxiosInstance.get.mockResolvedValue({ data: mockResponse });

      const result = await client.checkHealth();
      expect(result).toEqual(mockResponse);
      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/api/v1/health');
    });
  });

  describe('checkVersion', () => {
    it('should return version compatibility info for compatible versions', async () => {
      const mockHealthResponse = {
        status: 'healthy',
        version: '0.1.1',
        activeConnections: 5,
        queueSize: 10,
        activeChannels: 3,
      };

      mockAxiosInstance.get.mockResolvedValue({ data: mockHealthResponse });

      const result = await client.checkVersion();

      expect(result.clientVersion).toBe('0.1.1');
      expect(result.serverVersion).toBe('0.1.1');
      expect(result.compatible).toBe(true);
      expect(result.warnings).toHaveLength(0);
      expect(result.errors).toHaveLength(0);
    });

    it('should detect incompatible major versions', async () => {
      const mockHealthResponse = {
        status: 'healthy',
        version: '2.0.0',
        activeConnections: 5,
        queueSize: 10,
        activeChannels: 3,
      };

      mockAxiosInstance.get.mockResolvedValue({ data: mockHealthResponse });

      const result = await client.checkVersion();

      expect(result.compatible).toBe(false);
      expect(result.errors).toContain('Major version mismatch: client 0, server 2');
    });

    it('should warn on minor version differences', async () => {
      const mockHealthResponse = {
        status: 'healthy',
        version: '0.2.0',
        activeConnections: 5,
        queueSize: 10,
        activeChannels: 3,
      };

      mockAxiosInstance.get.mockResolvedValue({ data: mockHealthResponse });

      const result = await client.checkVersion();

      expect(result.compatible).toBe(true);
      expect(result.warnings).toContain('Minor version mismatch: client 1, server 2');
    });
  });

  describe('ask', () => {
    it('should send a question message', async () => {
      const mockResponse = {
        id: 'msg-123',
        channel: 'test-channel',
        sender_type: 'AGENT',
        content: { type: 'question', text: 'What is the answer?', timeout_seconds: 60 },
        timestamp: '2024-01-01T00:00:00Z',
      };

      mockAxiosInstance.post.mockResolvedValue({ data: mockResponse });

      const result = await client.ask('test-channel', 'What is the answer?');

      expect(result).toEqual(mockResponse);
      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/api/v1/messages', expect.objectContaining({
        channel: 'test-channel',
        sender_type: 'AGENT',
        content: expect.objectContaining({
          type: 'question',
          text: 'What is the answer?',
          timeout_seconds: 60,
        }),
      }));
    });
  });

  describe('authorize', () => {
    it('should send an authorization message', async () => {
      const mockResponse = {
        id: 'msg-124',
        channel: 'admin-channel',
        sender_type: 'AGENT',
        content: {
          type: 'authorization',
          action: 'Deploy to production',
          timeout_seconds: 300,
          context: { environment: 'prod' }
        },
        timestamp: '2024-01-01T00:00:00Z',
      };

      mockAxiosInstance.post.mockResolvedValue({ data: mockResponse });

      const result = await client.authorize('admin-channel', 'Deploy to production', 300, { environment: 'prod' });

      expect(result).toEqual(mockResponse);
      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/api/v1/messages', expect.objectContaining({
        channel: 'admin-channel',
        content: expect.objectContaining({
          type: 'authorization',
          action: 'Deploy to production',
          context: { environment: 'prod' },
        }),
      }));
    });
  });

  describe('say', () => {
    it('should send a notification message', async () => {
      const mockResponse = {
        id: 'msg-125',
        channel: 'general',
        sender_type: 'AGENT',
        content: { type: 'notification', text: 'Hello world', priority: 'normal' },
        timestamp: '2024-01-01T00:00:00Z',
      };

      mockAxiosInstance.post.mockResolvedValue({ data: mockResponse });

      const result = await client.say('general', 'Hello world', 'normal' as any);

      expect(result).toEqual(mockResponse);
      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/api/v1/messages', expect.objectContaining({
        channel: 'general',
        content: expect.objectContaining({
          type: 'notification',
          text: 'Hello world',
          priority: 'normal',
        }),
      }));
    });
  });

  describe('navigate', () => {
    it('should send a navigation message', async () => {
      const mockResponse = {
        id: 'msg-126',
        channel: 'ui-channel',
        sender_type: 'AGENT',
        content: { type: 'navigate', url: 'https://example.com' },
        timestamp: '2024-01-01T00:00:00Z',
      };

      mockAxiosInstance.post.mockResolvedValue({ data: mockResponse });

      const result = await client.navigate('ui-channel', 'https://example.com');

      expect(result).toEqual(mockResponse);
      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/api/v1/messages', expect.objectContaining({
        channel: 'ui-channel',
        content: expect.objectContaining({
          type: 'navigate',
          url: 'https://example.com',
        }),
      }));
    });
  });

  describe('getMessage', () => {
    it('should retrieve a message by ID', async () => {
      const mockResponse = {
        id: 'msg-123',
        channel: 'test-channel',
        sender_type: 'AGENT',
        content: { type: 'question', text: 'Test question', timeout_seconds: 60 },
        timestamp: '2024-01-01T00:00:00Z',
      };

      mockAxiosInstance.get.mockResolvedValue({ data: mockResponse });

      const result = await client.getMessage('msg-123');

      expect(result).toEqual(mockResponse);
      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/api/v1/messages/msg-123');
    });

    it('should throw ValidationError for 404 responses', async () => {
      const axiosError = {
        response: { status: 404, data: { error: 'Message not found' } },
        isAxiosError: true,
        message: 'Request failed with status code 404',
      };
      // Mock axios.isAxiosError to return true for this error
      const originalIsAxiosError = require('axios').isAxiosError;
      require('axios').isAxiosError = jest.fn(() => true);

      mockAxiosInstance.get.mockRejectedValue(axiosError);

      try {
        await expect(client.getMessage('nonexistent')).rejects.toThrow('Message not found');
      } finally {
        // Restore original function
        require('axios').isAxiosError = originalIsAxiosError;
      }
    });
  });

  describe('respond', () => {
    it('should send a response to a message', async () => {
      const originalMessage = {
        id: 'msg-123',
        channel: 'test-channel',
        sender_type: 'AGENT',
        content: { type: 'question', text: 'Test question', timeout_seconds: 60 },
        timestamp: '2024-01-01T00:00:00Z',
      };

      const responseMessage = {
        id: 'msg-124',
        channel: 'test-channel',
        sender_type: 'HUMAN',
        correlation_id: 'msg-123',
        content: { type: 'response', answer: 'Yes', response_type: 'text' },
        timestamp: '2024-01-01T00:01:00Z',
      };

      mockAxiosInstance.get.mockResolvedValue({ data: originalMessage });
      mockAxiosInstance.post.mockResolvedValue({ data: responseMessage });

      const result = await client.respond('msg-123', 'Yes', 'text');

      expect(result).toEqual(responseMessage);
      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/api/v1/messages/msg-123');
      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/api/v1/messages', expect.objectContaining({
        channel: 'test-channel',
        correlation_id: 'msg-123',
        content: expect.objectContaining({
          type: 'response',
          answer: 'Yes',
          response_type: 'text',
        }),
      }));
    });
  });
});
