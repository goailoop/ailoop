//! Message data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of message sender
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SenderType {
    #[serde(rename = "AGENT")]
    Agent,
    #[serde(rename = "HUMAN")]
    Human,
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "question")]
    Question {
        text: String,
        timeout_seconds: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<String>>,
    },
    #[serde(rename = "authorization")]
    Authorization {
        action: String,
        context: Option<serde_json::Value>,
        timeout_seconds: u32,
    },
    #[serde(rename = "notification")]
    Notification {
        text: String,
        #[serde(default)]
        priority: NotificationPriority,
    },
    #[serde(rename = "response")]
    Response {
        answer: Option<String>,
        response_type: ResponseType,
    },
    #[serde(rename = "navigate")]
    Navigate { url: String },
    #[serde(rename = "workflow_progress")]
    WorkflowProgress {
        execution_id: String,
        workflow_name: String,
        current_state: String,
        status: String,
        progress_percentage: Option<u8>,
    },
    #[serde(rename = "workflow_completed")]
    WorkflowCompleted {
        execution_id: String,
        workflow_name: String,
        final_status: String,
        duration_seconds: u64,
    },
    #[serde(rename = "stdout")]
    Stdout {
        execution_id: String,
        state_name: String,
        content: String,
        sequence: u64,
    },
    #[serde(rename = "stderr")]
    Stderr {
        execution_id: String,
        state_name: String,
        content: String,
        sequence: u64,
    },
    #[serde(rename = "task_create")]
    TaskCreate { task: Task },
    #[serde(rename = "task_update")]
    TaskUpdate {
        task_id: Uuid,
        state: TaskState,
        updated_at: DateTime<Utc>,
    },
    #[serde(rename = "task_dependency_add")]
    TaskDependencyAdd {
        task_id: Uuid,
        depends_on: Uuid,
        dependency_type: DependencyType,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "task_dependency_remove")]
    TaskDependencyRemove {
        task_id: Uuid,
        depends_on: Uuid,
        timestamp: DateTime<Utc>,
    },
}

/// Priority levels for notifications
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum NotificationPriority {
    #[default]
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "urgent")]
    Urgent,
}

/// Types of responses to questions/authorizations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResponseType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "authorization_approved")]
    AuthorizationApproved,
    #[serde(rename = "authorization_denied")]
    AuthorizationDenied,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// Task state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Pending,
    Done,
    Abandoned,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Pending => write!(f, "pending"),
            TaskState::Done => write!(f, "done"),
            TaskState::Abandoned => write!(f, "abandoned"),
        }
    }
}

/// Dependency type between tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Blocks,
    Related,
    Parent,
}

/// Task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub state: TaskState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assignee: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub depends_on: Vec<Uuid>,
    pub blocking_for: Vec<Uuid>,
    pub blocked: bool,
    pub dependency_type: Option<DependencyType>,
}

impl Task {
    pub fn new(title: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            state: TaskState::Pending,
            created_at: now,
            updated_at: now,
            assignee: None,
            metadata: None,
            depends_on: Vec::new(),
            blocking_for: Vec::new(),
            blocked: false,
            dependency_type: None,
        }
    }

    pub fn with_assignee(mut self, assignee: String) -> Self {
        self.assignee = Some(assignee);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_dependencies(mut self, depends_on: Vec<Uuid>) -> Self {
        self.depends_on = depends_on;
        self
    }
}

/// Core message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: Uuid,
    /// Channel name (validated)
    pub channel: String,
    /// Type of sender
    pub sender_type: SenderType,
    /// Message content/payload
    pub content: MessageContent,
    /// Creation timestamp
    pub timestamp: DateTime<Utc>,
    /// Links related messages (optional)
    pub correlation_id: Option<Uuid>,
    /// Extended metadata for agent-specific and client tracking information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    /// Create a new message
    pub fn new(channel: String, sender_type: SenderType, content: MessageContent) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel,
            sender_type,
            content,
            timestamp: Utc::now(),
            correlation_id: None,
            metadata: None,
        }
    }

    /// Create a response message linked to another message
    pub fn response(channel: String, content: MessageContent, correlation_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel,
            sender_type: SenderType::Human,
            content,
            timestamp: Utc::now(),
            correlation_id: Some(correlation_id),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let content = MessageContent::Question {
            text: "What is the answer?".to_string(),
            timeout_seconds: 60,
            choices: None,
        };

        let message = Message::new("test-channel".to_string(), SenderType::Agent, content);

        assert_eq!(message.channel, "test-channel");
        assert!(matches!(message.sender_type, SenderType::Agent));
        assert!(message.correlation_id.is_none());
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test Task".to_string(), "Test Description".to_string());

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "Test Description");
        assert_eq!(task.state, TaskState::Pending);
        assert!(!task.blocked);
        assert!(task.depends_on.is_empty());
        assert!(task.blocking_for.is_empty());
    }

    #[test]
    fn test_task_with_assignee() {
        let task = Task::new("Test Task".to_string(), "Test Description".to_string())
            .with_assignee("user@example.com".to_string());

        assert_eq!(task.assignee, Some("user@example.com".to_string()));
    }

    #[test]
    fn test_task_with_metadata() {
        let metadata = serde_json::json!({"key": "value"});
        let task = Task::new("Test Task".to_string(), "Test Description".to_string())
            .with_metadata(metadata.clone());

        assert_eq!(task.metadata, Some(metadata));
    }

    #[test]
    fn test_task_with_dependencies() {
        let parent_id = Uuid::new_v4();
        let task = Task::new("Test Task".to_string(), "Test Description".to_string())
            .with_dependencies(vec![parent_id]);

        assert_eq!(task.depends_on.len(), 1);
        assert_eq!(task.depends_on[0], parent_id);
    }

    #[test]
    fn test_task_state_serialization() {
        let states = vec![TaskState::Pending, TaskState::Done, TaskState::Abandoned];

        for state in states {
            let serialized = serde_json::to_string(&state).unwrap();
            let deserialized: TaskState = serde_json::from_str(&serialized).unwrap();
            assert_eq!(state, deserialized);
        }
    }

    #[test]
    fn test_dependency_type_serialization() {
        let types = vec![
            DependencyType::Blocks,
            DependencyType::Related,
            DependencyType::Parent,
        ];

        for dep_type in types {
            let serialized = serde_json::to_string(&dep_type).unwrap();
            let deserialized: DependencyType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(dep_type, deserialized);
        }
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new("Test Task".to_string(), "Test Description".to_string());
        let serialized = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&serialized).unwrap();

        assert_eq!(task.title, deserialized.title);
        assert_eq!(task.description, deserialized.description);
        assert_eq!(task.state, deserialized.state);
    }

    #[test]
    fn test_task_create_content() {
        let task = Task::new("Test Task".to_string(), "Test Description".to_string());
        let content = MessageContent::TaskCreate { task: task.clone() };

        let serialized = serde_json::to_string(&content).unwrap();
        assert!(serialized.contains("\"task_create\""));
        assert!(serialized.contains(&task.title));
    }
}
