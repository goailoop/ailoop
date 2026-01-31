// AiloopClient for TypeScript SDK

import axios, { AxiosInstance } from 'axios';
import WebSocket from 'isomorphic-ws';
import {
  AiloopClientOptions,
  ConnectionState,
  HealthResponse,
  VersionInfo,
  ConnectionError,
  ValidationError,
  TimeoutError,
  MessageHandler,
  ConnectionHandler
} from './types';
import { Message, MessageFactory, ResponseType, NotificationPriority, Task, TaskState, DependencyType } from './models';

export class AiloopClient {
  private httpClient: AxiosInstance;
  private wsClient?: WebSocket;
  private options: Required<AiloopClientOptions>;
  private connectionState: ConnectionState = {
    connected: false,
    channels: []
  };
  private messageHandlers: MessageHandler[] = [];
  private connectionHandlers: ConnectionHandler[] = [];
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private baseReconnectDelay = 1000; // 1 second
  private wsUrl?: string;

  constructor(options: AiloopClientOptions = {}) {
    this.options = {
      baseURL: options.baseURL || 'http://localhost:8080',
      timeout: options.timeout || 30000,
      maxRetries: options.maxRetries || 5,
      retryDelay: options.retryDelay || 1000,
      ...options
    };

    this.httpClient = axios.create({
      baseURL: this.options.baseURL,
      timeout: this.options.timeout,
    });
  }

  private async sendMessage(message: Omit<Message, 'id' | 'timestamp'>): Promise<Message> {
    try {
      const response = await this.httpClient.post('/api/v1/messages', message);
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 400) {
          throw new ValidationError(`Invalid message: ${error.response.data?.error || error.message}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else if (error instanceof Error && error.message.includes('timeout')) {
        throw new TimeoutError('Request timed out');
      } else {
        throw new ConnectionError(`Failed to send message: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async ask(
    channel: string,
    question: string,
    timeoutSeconds: number = 60,
    choices?: string[]
  ): Promise<Message> {
    const message = MessageFactory.createQuestion(channel, question, timeoutSeconds, choices);
    return await this.sendMessage(message);
  }

  async authorize(
    channel: string,
    action: string,
    timeoutSeconds: number = 300,
    context?: Record<string, any>
  ): Promise<Message> {
    const message = MessageFactory.createAuthorization(channel, action, timeoutSeconds, context);
    return await this.sendMessage(message);
  }

  async say(
    channel: string,
    text: string,
    priority: NotificationPriority = 'normal'
  ): Promise<Message> {
    const message = MessageFactory.createNotification(channel, text, priority);
    return await this.sendMessage(message);
  }

  async navigate(
    channel: string,
    url: string
  ): Promise<Message> {
    const message = MessageFactory.createNavigate(channel, url);
    return await this.sendMessage(message);
  }

  async createTask(
    channel: string,
    title: string,
    description: string,
    assignee?: string,
    metadata?: Record<string, any>
  ): Promise<Task> {
    try {
      const response = await this.httpClient.post('/api/v1/tasks', {
        title,
        description,
        channel,
        assignee,
        metadata,
      });
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
      } else {
        throw new ConnectionError(`Failed to create task: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async updateTask(
    channel: string,
    taskId: string,
    state: TaskState
  ): Promise<Task> {
    try {
      const response = await this.httpClient.put(`/api/v1/tasks/${taskId}`, { state });
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 404) {
          throw new ValidationError(`Task not found: ${taskId}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else {
        throw new ConnectionError(`Failed to update task: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async listTasks(channel: string, state?: TaskState): Promise<Task[]> {
    try {
      const params: any = { channel };
      if (state) {
        params.state = state;
      }
      const response = await this.httpClient.get('/api/v1/tasks', { params });
      return response.data.tasks || [];
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
      } else {
        throw new ConnectionError(`Failed to list tasks: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async getTask(taskId: string): Promise<Task> {
    try {
      const response = await this.httpClient.get(`/api/v1/tasks/${taskId}`);
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 404) {
          throw new ValidationError(`Task not found: ${taskId}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else {
        throw new ConnectionError(`Failed to get task: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async addDependency(
    taskId: string,
    dependsOn: string,
    type: DependencyType = 'blocks'
  ): Promise<void> {
    try {
      await this.httpClient.post(`/api/v1/tasks/${taskId}/dependencies`, {
        child_id: taskId,
        parent_id: dependsOn,
        dependency_type: type,
      });
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 400) {
          throw new ValidationError(`Invalid dependency: ${error.response?.data?.error || error.message}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else {
        throw new ConnectionError(`Failed to add dependency: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async removeDependency(taskId: string, dependsOn: string): Promise<void> {
    try {
      await this.httpClient.delete(`/api/v1/tasks/${taskId}/dependencies/${dependsOn}`);
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 404) {
          throw new ValidationError(`Dependency not found between ${taskId} and ${dependsOn}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else {
        throw new ConnectionError(`Failed to remove dependency: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async getReadyTasks(channel: string): Promise<Task[]> {
    try {
      const response = await this.httpClient.get('/api/v1/tasks/ready', {
        params: { channel },
      });
      return response.data.tasks || [];
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
      } else {
        throw new ConnectionError(`Failed to get ready tasks: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async getBlockedTasks(channel: string): Promise<Task[]> {
    try {
      const response = await this.httpClient.get('/api/v1/tasks/blocked', {
        params: { channel },
      });
      return response.data.tasks || [];
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
      } else {
        throw new ConnectionError(`Failed to get blocked tasks: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async getDependencyGraph(taskId: string): Promise<{ task: Task; parents: Task[]; children: Task[] }> {
    try {
      const response = await this.httpClient.get(`/api/v1/tasks/${taskId}/graph`);
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 404) {
          throw new ValidationError(`Task not found: ${taskId}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else {
        throw new ConnectionError(`Failed to get dependency graph: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }


  async getMessage(id: string): Promise<Message> {
    try {
      const response = await this.httpClient.get(`/api/v1/messages/${id}`);
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.response?.status === 404) {
          throw new ValidationError(`Message not found: ${id}`);
        } else {
          throw new ConnectionError(`HTTP error ${error.response?.status}: ${error.response?.data?.error || error.message}`);
        }
      } else if (error instanceof Error && error.message.includes('timeout')) {
        throw new TimeoutError('Request timed out');
      } else {
        throw new ConnectionError(`Failed to get message: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  async respond(messageId: string, answer?: string, responseType: ResponseType = 'text'): Promise<Message> {
    // First get the original message to know the channel
    const originalMessage = await this.getMessage(messageId);

    const response = MessageFactory.createResponse(messageId, answer, responseType);
    // Add the channel from the original message
    const responseWithChannel = {
      ...response,
      channel: originalMessage.channel
    };

    return await this.sendMessage(responseWithChannel);
  }

  // WebSocket methods
  async connect(): Promise<void> {
    // Check version compatibility before connecting
    await this.ensureVersionCompatibility();

    if (this.connectionState.connected) {
      return; // Already connected
    }

    // Convert HTTP URL to WebSocket URL
    const httpUrl = new URL(this.options.baseURL);
    const wsUrl = `ws://${httpUrl.host}/ws`;
    this.wsUrl = wsUrl;

    await this.connectWebSocket();
  }

  async disconnect(): Promise<void> {
    if (this.wsClient) {
      this.wsClient.close();
      this.wsClient = undefined!;
    }

    this.connectionState.connected = false;
    this.connectionState.channels = [];
    this.notifyConnectionHandlers({ type: 'disconnected' });
  }

  async subscribe(channel: string): Promise<void> {
    if (!this.wsClient || !this.connectionState.connected) {
      throw new ConnectionError('WebSocket not connected');
    }

    const subscribeMessage = JSON.stringify({
      type: 'subscribe',
      channel
    });

    this.wsClient.send(subscribeMessage);
    this.connectionState.channels.push(channel);
  }

  async unsubscribe(channel: string): Promise<void> {
    if (!this.wsClient || !this.connectionState.connected) {
      throw new ConnectionError('WebSocket not connected');
    }

    const unsubscribeMessage = JSON.stringify({
      type: 'unsubscribe',
      channel
    });

    this.wsClient.send(unsubscribeMessage);
    this.connectionState.channels = this.connectionState.channels.filter(c => c !== channel);
  }

  addMessageHandler(handler: MessageHandler): void {
    this.messageHandlers.push(handler);
  }

  addConnectionHandler(handler: ConnectionHandler): void {
    this.connectionHandlers.push(handler);
  }

  private async connectWebSocket(): Promise<void> {
    if (!this.wsUrl) {
      throw new ConnectionError('WebSocket URL not set');
    }

    return new Promise((resolve, reject) => {
      try {
        this.wsClient = new WebSocket(this.wsUrl!);

        this.wsClient.onopen = () => {
          this.connectionState.connected = true;
          this.reconnectAttempts = 0;
          this.notifyConnectionHandlers({ type: 'connected' });

          // Resubscribe to previously subscribed channels
          this.resubscribeChannels();

          resolve();
        };

        this.wsClient.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data.toString());
            this.notifyMessageHandlers(message);
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
          }
        };

        this.wsClient.onclose = () => {
          this.connectionState.connected = false;
          this.notifyConnectionHandlers({ type: 'disconnected' });

          // Attempt reconnection if not manually disconnected
          if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.scheduleReconnection();
          }
        };

        this.wsClient.onerror = (error) => {
          this.notifyConnectionHandlers({ type: 'error', error: error.toString() });

          if (!this.connectionState.connected) {
            reject(new ConnectionError(`WebSocket connection failed: ${error}`));
          }
        };

      } catch (error) {
        reject(new ConnectionError(`Failed to create WebSocket connection: ${error}`));
      }
    });
  }

  private resubscribeChannels(): void {
    if (!this.wsClient || !this.connectionState.connected) return;

    for (const channel of this.connectionState.channels) {
      const subscribeMessage = JSON.stringify({
        type: 'subscribe',
        channel
      });
      this.wsClient.send(subscribeMessage);
    }
  }

  private scheduleReconnection(): void {
    this.reconnectAttempts++;
    const delay = this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    setTimeout(() => {
      console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts}) in ${delay}ms...`);
      this.connectWebSocket().catch(error => {
        console.error('Reconnection failed:', error);
      });
    }, delay);
  }

  private notifyMessageHandlers(message: any): void {
    for (const handler of this.messageHandlers) {
      try {
        handler(message);
      } catch (error) {
        console.error('Message handler error:', error);
      }
    }
  }

  private notifyConnectionHandlers(event: { type: 'connected' | 'disconnected' | 'error'; error?: string }): void {
    for (const handler of this.connectionHandlers) {
      try {
        handler(event);
      } catch (error) {
        console.error('Connection handler error:', error);
      }
    }
  }

  getConnectionState(): ConnectionState {
    return { ...this.connectionState };
  }

  async checkHealth(): Promise<HealthResponse> {
    try {
      const response = await this.httpClient.get('/api/v1/health');
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new ConnectionError(`Health check failed: ${error.message}`);
      }
      throw new ConnectionError('Health check failed');
    }
  }

  async checkVersion(): Promise<VersionInfo> {
    try {
      const health = await this.checkHealth();
      const clientVersion = '0.1.1'; // TODO: Get from package.json
      const serverVersion = health.version;

      // Parse version numbers
      const clientParts = this.parseVersion(clientVersion);
      const serverParts = this.parseVersion(serverVersion);

      const warnings: string[] = [];
      const errors: string[] = [];

      // Check major version compatibility (must match)
      if (clientParts.major !== serverParts.major) {
        errors.push(`Major version mismatch: client ${clientParts.major}, server ${serverParts.major}`);
      }

      // Check minor version compatibility (warn on mismatch)
      if (clientParts.minor !== serverParts.minor) {
        warnings.push(`Minor version mismatch: client ${clientParts.minor}, server ${serverParts.minor}`);
      }

      // Check patch version compatibility (warn on mismatch)
      if (clientParts.patch !== serverParts.patch) {
        warnings.push(`Patch version mismatch: client ${clientParts.patch}, server ${serverParts.patch}`);
      }

      const compatible = errors.length === 0;

      return {
        clientVersion,
        serverVersion,
        compatible,
        warnings,
        errors
      };
    } catch (error) {
      throw new ConnectionError('Version check failed');
    }
  }

  private parseVersion(version: string): { major: number; minor: number; patch: number } {
    const parts = version.split('.');
    return {
      major: parts[0] ? parseInt(parts[0], 10) : 0,
      minor: parts[1] ? parseInt(parts[1], 10) : 0,
      patch: parts[2] ? parseInt(parts[2], 10) : 0,
    };
  }

  async ensureVersionCompatibility(): Promise<void> {
    const versionInfo = await this.checkVersion();

    // Log warnings
    versionInfo.warnings.forEach(warning => {
      console.warn(`Version compatibility warning: ${warning}`);
    });

    // Throw error for incompatible versions
    if (!versionInfo.compatible) {
      const errorMessage = `Incompatible server version: ${versionInfo.errors.join(', ')}`;
      throw new ValidationError(errorMessage);
    }
  }
}
