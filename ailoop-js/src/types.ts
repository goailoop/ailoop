// Common types for the ailoop TypeScript SDK

export interface AiloopClientOptions {
  baseURL?: string;
  timeout?: number;
  maxRetries?: number;
  retryDelay?: number;
}

export interface ConnectionState {
  connected: boolean;
  url?: string;
  channels: string[];
}

export interface HealthResponse {
  status: string;
  version: string;
  activeConnections: number;
  queueSize: number;
  activeChannels: number;
}

export interface VersionInfo {
  clientVersion: string;
  serverVersion: string;
  compatible: boolean;
  warnings: string[];
  errors: string[];
}

export interface WebSocketMessage {
  type: 'message' | 'subscribe' | 'unsubscribe' | 'connected' | 'disconnected' | 'error';
  channel?: string;
  data?: any;
  error?: string;
}

export type MessageHandler = (message: WebSocketMessage) => void | Promise<void>;
export type ConnectionHandler = (event: { type: 'connected' | 'disconnected' | 'error'; error?: string }) => void | Promise<void>;

export class AiloopError extends Error {
  constructor(message: string, public readonly code?: string) {
    super(message);
    this.name = 'AiloopError';
  }
}

export class ConnectionError extends AiloopError {
  constructor(message: string) {
    super(message, 'CONNECTION_ERROR');
    this.name = 'ConnectionError';
  }
}

export class ValidationError extends AiloopError {
  constructor(message: string) {
    super(message, 'VALIDATION_ERROR');
    this.name = 'ValidationError';
  }
}

export class TimeoutError extends AiloopError {
  constructor(message: string) {
    super(message, 'TIMEOUT_ERROR');
    this.name = 'TimeoutError';
  }
}
