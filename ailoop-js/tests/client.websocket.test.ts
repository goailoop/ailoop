import WebSocket from 'isomorphic-ws';
import { AiloopClient } from '../src/client';

jest.mock('isomorphic-ws');

describe('AiloopClient WebSocket Methods', () => {
  let client: AiloopClient;
  let mockWebSocket: jest.Mocked<WebSocket>;

  beforeEach(() => {
    jest.clearAllMocks();
    client = new AiloopClient({ baseURL: 'http://localhost:8080' });

    // Mock axios health/version checks
    const mockHttpClient = {
      get: jest.fn().mockResolvedValue({
        data: { status: 'healthy', version: '0.1.1', activeConnections: 0, queueSize: 0, activeChannels: 0 }
      }),
    };
    (client as any).httpClient = mockHttpClient;
  });

  afterEach(async () => {
    if (client) {
      await client.disconnect().catch(() => {});
    }
  });

  describe('connect', () => {
    it('should establish WebSocket connection', async () => {
      mockWebSocket = {
        send: jest.fn(),
        close: jest.fn(),
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        onopen: jest.fn(),
        onmessage: jest.fn(),
        onclose: jest.fn(),
        onerror: jest.fn(),
        readyState: WebSocket.OPEN,
      } as any;

      (WebSocket as jest.MockedClass<typeof WebSocket>).mockImplementation(() => mockWebSocket);

      // Mock successful connection
      setTimeout(() => {
        mockWebSocket.onopen!({} as any);
      }, 10);

      await client.connect();

      expect(client.getConnectionState().connected).toBe(true);
      expect(WebSocket).toHaveBeenCalledWith('ws://localhost:8080/ws');
    });

    it('should check version compatibility before connecting', async () => {
      const versionCheckSpy = jest.spyOn(client as any, 'ensureVersionCompatibility');

      mockWebSocket = {
        send: jest.fn(),
        close: jest.fn(),
        onopen: jest.fn(),
        onmessage: jest.fn(),
        onclose: jest.fn(),
        onerror: jest.fn(),
        readyState: WebSocket.OPEN,
      } as any;

      (WebSocket as jest.MockedClass<typeof WebSocket>).mockImplementation(() => mockWebSocket);

      setTimeout(() => {
        mockWebSocket.onopen!({} as any);
      }, 10);

      await client.connect();

      expect(versionCheckSpy).toHaveBeenCalled();
    });
  });

  describe('disconnect', () => {
    it('should close WebSocket connection', async () => {
      mockWebSocket = {
        close: jest.fn(),
        onopen: jest.fn(),
        onmessage: jest.fn(),
        onclose: jest.fn(),
        onerror: jest.fn(),
        readyState: WebSocket.OPEN,
      } as any;

      (client as any).wsClient = mockWebSocket;
      (client as any).connectionState.connected = true;

      await client.disconnect();

      expect(mockWebSocket.close).toHaveBeenCalled();
      expect(client.getConnectionState().connected).toBe(false);
    });
  });

  describe('subscribe', () => {
    it('should send subscribe message', async () => {
      mockWebSocket = {
        send: jest.fn(),
        onopen: jest.fn(),
        onmessage: jest.fn(),
        onclose: jest.fn(),
        onerror: jest.fn(),
        readyState: WebSocket.OPEN,
      } as any;

      (client as any).wsClient = mockWebSocket;
      (client as any).connectionState.connected = true;

      await client.subscribe('test-channel');

      expect(mockWebSocket.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'subscribe', channel: 'test-channel' })
      );
      expect(client.getConnectionState().channels).toContain('test-channel');
    });

    it('should throw error when not connected', async () => {
      (client as any).connectionState.connected = false;

      await expect(client.subscribe('test-channel')).rejects.toThrow('WebSocket not connected');
    });
  });

  describe('unsubscribe', () => {
    it('should send unsubscribe message', async () => {
      mockWebSocket = {
        send: jest.fn(),
        onopen: jest.fn(),
        onmessage: jest.fn(),
        onclose: jest.fn(),
        onerror: jest.fn(),
        readyState: WebSocket.OPEN,
      } as any;

      (client as any).wsClient = mockWebSocket;
      (client as any).connectionState.connected = true;
      (client as any).connectionState.channels = ['test-channel', 'other-channel'];

      await client.unsubscribe('test-channel');

      expect(mockWebSocket.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'unsubscribe', channel: 'test-channel' })
      );
      expect(client.getConnectionState().channels).not.toContain('test-channel');
      expect(client.getConnectionState().channels).toContain('other-channel');
    });
  });

  describe('message handlers', () => {
    it('should add and call message handlers', () => {
      const mockHandler = jest.fn();

      client.addMessageHandler(mockHandler);

      // Simulate calling handlers (normally done internally)
      const mockMessage = { type: 'message', data: 'test' };
      (client as any).notifyMessageHandlers(mockMessage);

      expect(mockHandler).toHaveBeenCalledWith(mockMessage);
    });

    it('should handle handler errors gracefully', () => {
      const mockHandler = jest.fn().mockImplementation(() => {
        throw new Error('Handler error');
      });
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      client.addMessageHandler(mockHandler);

      (client as any).notifyMessageHandlers({ type: 'message' });

      expect(mockHandler).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });
  });

  describe('connection handlers', () => {
    it('should add and call connection handlers', () => {
      const mockHandler = jest.fn();

      client.addConnectionHandler(mockHandler);

      // Simulate calling handlers (normally done internally)
      const mockEvent = { type: 'connected' as const };
      (client as any).notifyConnectionHandlers(mockEvent);

      expect(mockHandler).toHaveBeenCalledWith(mockEvent);
    });
  });

  describe('reconnection', () => {
    it('should attempt reconnection on connection close', async () => {
      mockWebSocket = {
        send: jest.fn(),
        close: jest.fn(),
        onopen: jest.fn(),
        onmessage: jest.fn(),
        onclose: jest.fn(),
        onerror: jest.fn(),
        readyState: WebSocket.OPEN,
      } as any;

      (WebSocket as jest.MockedClass<typeof WebSocket>).mockImplementation(() => mockWebSocket);

      // First connection
      setTimeout(() => {
        mockWebSocket.onopen!({} as any);
      }, 10);

      await client.connect();
      expect(client.getConnectionState().connected).toBe(true);

      // Simulate connection close
      mockWebSocket.onclose!({} as any);

      // Should attempt reconnection
      expect((client as any).reconnectAttempts).toBeGreaterThan(0);
    });

    it('should increment reconnection attempts counter', () => {
      (client as any).reconnectAttempts = 4;
      (client as any).maxReconnectAttempts = 5;

      // Schedule reconnection
      (client as any).scheduleReconnection();

      // reconnectAttempts should increment
      expect((client as any).reconnectAttempts).toBe(5);
    });
  });
});
