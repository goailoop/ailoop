// Test setup for jest
import { jest } from '@jest/globals';

// Mock WebSocket for tests
global.WebSocket = jest.fn().mockImplementation(() => ({
  addEventListener: jest.fn(),
  removeEventListener: jest.fn(),
  dispatchEvent: jest.fn(),
  send: jest.fn(),
  close: jest.fn(),
  readyState: 1, // OPEN
  CONNECTING: 0,
  OPEN: 1,
  CLOSING: 2,
  CLOSED: 3,
})) as any;

// Mock isomorphic-ws to use the global WebSocket
jest.mock('isomorphic-ws', () => ({
  __esModule: true,
  default: global.WebSocket,
}));
