// Message models for ailoop TypeScript SDK
// These types match the Rust message serialization format exactly

export type SenderType = 'HUMAN' | 'AGENT' | 'SYSTEM';

export type ResponseType = 'text' | 'authorization_approved' | 'authorization_denied' | 'timeout' | 'cancelled';

export type NotificationPriority = 'low' | 'normal' | 'high' | 'urgent';

export interface Message {
  id: string;
  channel: string;
  sender_type: SenderType;
  content: MessageContent;
  timestamp: string;
  correlation_id?: string;
  metadata?: Record<string, any>;
}

export type MessageContent =
  | QuestionContent
  | AuthorizationContent
  | NotificationContent
  | ResponseContent
  | NavigateContent;

export interface QuestionContent {
  type: 'question';
  text: string;
  timeout_seconds: number;
  choices?: string[] | undefined;
}

export interface AuthorizationContent {
  type: 'authorization';
  action: string;
  timeout_seconds: number;
  context?: Record<string, any> | undefined;
}

export interface NotificationContent {
  type: 'notification';
  text: string;
  priority: NotificationPriority;
}

export interface ResponseContent {
  type: 'response';
  answer?: string | undefined;
  response_type: ResponseType;
}

export interface NavigateContent {
  type: 'navigate';
  url: string;
}

// Factory methods for creating messages
export class MessageFactory {
  static createQuestion(
    channel: string,
    text: string,
    timeoutSeconds: number = 60,
    choices?: string[]
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'question',
        text,
        timeout_seconds: timeoutSeconds,
        choices
      }
    };
  }

  static createAuthorization(
    channel: string,
    action: string,
    timeoutSeconds: number = 300,
    context?: Record<string, any>
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'authorization',
        action,
        timeout_seconds: timeoutSeconds,
        context
      }
    };
  }

  static createNotification(
    channel: string,
    text: string,
    priority: NotificationPriority = 'normal'
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'notification',
        text,
        priority
      }
    };
  }

  static createResponse(
    correlationId: string,
    answer?: string,
    responseType: ResponseType = 'text'
  ): Omit<Message, 'id' | 'timestamp' | 'channel'> {
    return {
      sender_type: 'HUMAN',
      correlation_id: correlationId,
      content: {
        type: 'response',
        answer,
        response_type: responseType
      }
    };
  }

  static createNavigate(
    channel: string,
    url: string
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'navigate',
        url
      }
    };
  }
}
