// Message models for ailoop TypeScript SDK
// These types match the Rust message serialization format exactly

export type SenderType = 'HUMAN' | 'AGENT' | 'SYSTEM';

export type ResponseType = 'text' | 'authorization_approved' | 'authorization_denied' | 'timeout' | 'cancelled';

export type NotificationPriority = 'low' | 'normal' | 'high' | 'urgent';

export type TaskState = 'pending' | 'done' | 'abandoned';

export type DependencyType = 'blocks' | 'related' | 'parent';

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
  | NavigateContent
  | TaskCreateContent
  | TaskUpdateContent
  | TaskDependencyAddContent
  | TaskDependencyRemoveContent;

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

export interface TaskCreateContent {
  type: 'task_create';
  task: Task;
}

export interface TaskUpdateContent {
  type: 'task_update';
  task_id: string;
  state: TaskState;
  updated_at: string;
}

export interface TaskDependencyAddContent {
  type: 'task_dependency_add';
  task_id: string;
  depends_on: string;
  dependency_type: DependencyType;
  timestamp: string;
}

export interface TaskDependencyRemoveContent {
  type: 'task_dependency_remove';
  task_id: string;
  depends_on: string;
  timestamp: string;
}

export interface Task {
  id: string;
  title: string;
  description: string;
  state: TaskState;
  created_at: string;
  updated_at: string;
  assignee?: string;
  metadata?: Record<string, any>;
  depends_on: string[];
  blocking_for: string[];
  blocked: boolean;
  dependency_type?: DependencyType;
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

  static createTaskCreate(
    channel: string,
    task: Task
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'task_create',
        task
      }
    };
  }

  static createTaskUpdate(
    channel: string,
    task_id: string,
    state: TaskState
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'task_update',
        task_id,
        state,
        updated_at: new Date().toISOString()
      }
    };
  }

  static createTaskDependencyAdd(
    channel: string,
    task_id: string,
    depends_on: string,
    dependency_type: DependencyType
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'task_dependency_add',
        task_id,
        depends_on,
        dependency_type,
        timestamp: new Date().toISOString()
      }
    };
  }

  static createTaskDependencyRemove(
    channel: string,
    task_id: string,
    depends_on: string
  ): Omit<Message, 'id' | 'timestamp'> {
    return {
      channel,
      sender_type: 'AGENT',
      content: {
        type: 'task_dependency_remove',
        task_id,
        depends_on,
        timestamp: new Date().toISOString()
      }
    };
  }
}
