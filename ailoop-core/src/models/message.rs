//! Message data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of message sender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SenderType {
    #[serde(rename = "AGENT")]
    Agent,
    #[serde(rename = "HUMAN")]
    Human,
}

/// A single selectable option within a Decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionOption {
    /// Stable machine-readable identifier, unique within the Decision. Non-empty.
    pub id: String,
    /// Human-readable label shown in UI and TTY.
    pub label: String,
    /// Optional markdown body rendered below the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_markdown: Option<String>,
}

/// Agent's recommendation within a Decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionRecommendation {
    /// MUST reference an id present in Decision.options.
    pub option_id: String,
    /// Optional markdown rationale for the recommendation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale_markdown: Option<String>,
}

/// Validate Decision fields. Returns Err with a human-readable message on failure.
pub fn validate_decision(
    options: &[DecisionOption],
    recommendation: &Option<DecisionRecommendation>,
) -> Result<(), String> {
    if options.len() < 2 {
        return Err(format!(
            "DECISION_TOO_FEW_OPTIONS: need at least 2 options, got {}",
            options.len()
        ));
    }
    let mut seen_ids = std::collections::HashSet::new();
    for opt in options {
        if opt.id.is_empty() {
            return Err("DECISION_EMPTY_OPTION_ID: option id must not be empty".into());
        }
        if !seen_ids.insert(opt.id.clone()) {
            return Err(format!(
                "DECISION_DUPLICATE_OPTION_ID: duplicate id '{}'",
                opt.id
            ));
        }
    }
    if let Some(rec) = recommendation {
        if !seen_ids.contains(&rec.option_id) {
            return Err(format!(
                "DECISION_INVALID_RECOMMENDATION: option_id '{}' not in options",
                rec.option_id
            ));
        }
    }
    Ok(())
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    /// Structured multi-option decision prompt for human selection.
    #[serde(rename = "decision")]
    Decision {
        /// Agent-assigned stable identifier for this decision.
        decision_id: String,
        /// Short summary / question shown as heading.
        summary: String,
        /// Optional markdown context block rendered above options.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_markdown: Option<String>,
        /// Selectable options. MUST have length >= 2 with unique ids.
        options: Vec<DecisionOption>,
        /// Agent's recommended option. option_id MUST be in options.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recommendation: Option<DecisionRecommendation>,
        /// Seconds until timeout. 0 = use server default.
        timeout_seconds: u32,
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
        let content = MessageContent::Decision {
            decision_id: "test-decision".to_string(),
            summary: "What is the answer?".to_string(),
            context_markdown: None,
            options: vec![
                DecisionOption {
                    id: "a".to_string(),
                    label: "Option A".to_string(),
                    detail_markdown: None,
                },
                DecisionOption {
                    id: "b".to_string(),
                    label: "Option B".to_string(),
                    detail_markdown: None,
                },
            ],
            recommendation: None,
            timeout_seconds: 60,
        };

        let message = Message::new("test-channel".to_string(), SenderType::Agent, content);

        assert_eq!(message.channel, "test-channel");
        assert!(matches!(message.sender_type, SenderType::Agent));
        assert!(message.correlation_id.is_none());
    }

    #[test]
    fn test_decision_round_trip() {
        let content = MessageContent::Decision {
            decision_id: "deploy-strategy".to_string(),
            summary: "Which deployment strategy?".to_string(),
            context_markdown: Some("Error rate: **0.3%**".to_string()),
            options: vec![
                DecisionOption {
                    id: "blue-green".to_string(),
                    label: "Blue/Green".to_string(),
                    detail_markdown: Some("Zero-downtime swap".to_string()),
                },
                DecisionOption {
                    id: "canary".to_string(),
                    label: "Canary (10%)".to_string(),
                    detail_markdown: None,
                },
            ],
            recommendation: Some(DecisionRecommendation {
                option_id: "blue-green".to_string(),
                rationale_markdown: Some("SLO budget is tight".to_string()),
            }),
            timeout_seconds: 300,
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"decision\""));
        let restored: MessageContent = serde_json::from_str(&json).unwrap();
        if let MessageContent::Decision {
            decision_id,
            summary,
            options,
            recommendation,
            ..
        } = restored
        {
            assert_eq!(decision_id, "deploy-strategy");
            assert_eq!(summary, "Which deployment strategy?");
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].id, "blue-green");
            assert_eq!(options[1].id, "canary");
            let rec = recommendation.unwrap();
            assert_eq!(rec.option_id, "blue-green");
        } else {
            panic!("Expected Decision variant");
        }
    }

    #[test]
    fn test_validate_decision_too_few_options() {
        let options = vec![DecisionOption {
            id: "a".to_string(),
            label: "A".to_string(),
            detail_markdown: None,
        }];
        let result = validate_decision(&options, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DECISION_TOO_FEW_OPTIONS"));
    }

    #[test]
    fn test_validate_decision_duplicate_option_id() {
        let options = vec![
            DecisionOption {
                id: "a".to_string(),
                label: "A".to_string(),
                detail_markdown: None,
            },
            DecisionOption {
                id: "a".to_string(),
                label: "A2".to_string(),
                detail_markdown: None,
            },
        ];
        let result = validate_decision(&options, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DECISION_DUPLICATE_OPTION_ID"));
    }

    #[test]
    fn test_validate_decision_invalid_recommendation() {
        let options = vec![
            DecisionOption {
                id: "a".to_string(),
                label: "A".to_string(),
                detail_markdown: None,
            },
            DecisionOption {
                id: "b".to_string(),
                label: "B".to_string(),
                detail_markdown: None,
            },
        ];
        let rec = Some(DecisionRecommendation {
            option_id: "nonexistent".to_string(),
            rationale_markdown: None,
        });
        let result = validate_decision(&options, &rec);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("DECISION_INVALID_RECOMMENDATION"));
    }

    #[test]
    fn test_validate_decision_valid() {
        let options = vec![
            DecisionOption {
                id: "a".to_string(),
                label: "A".to_string(),
                detail_markdown: None,
            },
            DecisionOption {
                id: "b".to_string(),
                label: "B".to_string(),
                detail_markdown: None,
            },
        ];
        let rec = Some(DecisionRecommendation {
            option_id: "a".to_string(),
            rationale_markdown: None,
        });
        let result = validate_decision(&options, &rec);
        assert!(result.is_ok());
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
