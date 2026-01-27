//! Workflow orchestration data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Execution status for workflow instances
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Workflow is actively executing
    Running,
    /// Waiting for human approval
    ApprovalPending,
    /// Successfully reached terminal state
    Completed,
    /// Failed and cannot proceed
    Failed,
    /// Workflow or state timed out
    Timeout,
    /// Approval was denied
    Denied,
    /// Manually cancelled by operator
    Cancelled,
}

/// Workflow definition - reusable template for executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique workflow identifier
    pub name: String,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Name of the starting state
    pub initial_state: String,
    /// List of states that end workflow execution
    pub terminal_states: Vec<String>,
    /// Map of state name to state definition
    pub states: HashMap<String, WorkflowState>,
    /// Default configuration for all states
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<DefaultConfiguration>,
}

/// Default configuration applied to all states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfiguration {
    /// Default timeout in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u32>,
    /// Default retry policy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicy>,
}

/// Individual workflow state definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// State name (unique within workflow)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Bash command to execute
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Maximum execution time in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u32>,
    /// Whether human approval is required
    #[serde(default)]
    pub requires_approval: bool,
    /// Approval wait time in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_timeout: Option<u32>,
    /// Description shown in approval request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_description: Option<String>,
    /// Retry configuration for this state
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicy>,
    /// State transition mapping
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transitions: Option<TransitionRules>,
    /// Timeout behavior for approval
    #[serde(default = "default_timeout_behavior")]
    pub timeout_behavior: TimeoutBehavior,
}

fn default_timeout_behavior() -> TimeoutBehavior {
    TimeoutBehavior::DenyAndFail
}

/// Timeout behavior for approval requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutBehavior {
    /// Deny and fail the workflow
    DenyAndFail,
    /// Deny and continue with denial transition
    DenyAndContinue,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (1-10)
    pub max_attempts: u32,
    /// Delay before first retry in seconds (1-300)
    pub initial_delay_seconds: u32,
    /// Whether to use exponential backoff
    #[serde(default)]
    pub exponential_backoff: bool,
    /// Multiplier for exponential backoff (1.0-10.0)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

/// Transition rules for different outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRules {
    /// Next state on success
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    /// Next state on failure
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// Next state on timeout
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    /// Next state on approval denial
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_denied: Option<String>,
}

/// Runtime workflow execution instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionInstance {
    /// Unique execution identifier
    pub id: Uuid,
    /// Name of the workflow definition
    pub workflow_name: String,
    /// Current/last state in execution
    pub current_state: String,
    /// Overall execution status
    pub status: ExecutionStatus,
    /// When workflow execution began
    pub started_at: DateTime<Utc>,
    /// When workflow execution finished
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Identity of user/system that started workflow
    pub initiator: String,
    /// Optional execution context
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// State transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Unique transition ID
    pub id: Uuid,
    /// Foreign key to workflow execution
    pub execution_id: Uuid,
    /// Previous state name (None for initial state)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_state: Option<String>,
    /// New state name
    pub to_state: String,
    /// Reason for transition
    pub transition_type: TransitionType,
    /// When transition occurred
    pub timestamp: DateTime<Utc>,
    /// Time spent in from_state (milliseconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Command exit code if applicable
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Additional context
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Reason for state transition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    /// Starting the workflow
    Initial,
    /// Command succeeded (exit code 0)
    Success,
    /// Command failed (non-zero exit code)
    Failure,
    /// State execution exceeded timeout
    Timeout,
    /// Approval was granted
    ApprovalGranted,
    /// Approval was denied
    ApprovalDenied,
    /// Approval request timed out
    ApprovalTimeout,
    /// Retrying after failure
    Retry,
    /// Manual state change
    Manual,
}

/// Execution output chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    /// Unique output record ID
    pub id: Uuid,
    /// Foreign key to workflow execution
    pub execution_id: Uuid,
    /// Name of state that produced output
    pub state_name: String,
    /// Stream type
    pub output_type: OutputType,
    /// Sequential chunk number
    pub chunk_sequence: u64,
    /// Output chunk data
    pub content: Vec<u8>,
    /// When chunk was captured
    pub timestamp: DateTime<Utc>,
}

/// Output stream type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    /// Standard output stream
    Stdout,
    /// Standard error stream
    Stderr,
}

/// Approval request record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique approval request identifier
    pub id: Uuid,
    /// Foreign key to workflow execution
    pub execution_id: Uuid,
    /// Name of state requiring approval
    pub state_name: String,
    /// Human-readable action description
    pub action_description: String,
    /// Current request status
    pub status: ApprovalStatus,
    /// When approval was requested
    pub requested_at: DateTime<Utc>,
    /// When response was received
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responded_at: Option<DateTime<Utc>>,
    /// Identity of user who responded
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responder: Option<String>,
    /// How long to wait for response (seconds)
    pub timeout_seconds: u32,
    /// Timeout behavior when no response
    pub timeout_behavior: TimeoutBehavior,
    /// Optional context for approval decision
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Approval request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Waiting for operator response
    Pending,
    /// Operator granted approval
    Approved,
    /// Operator denied approval
    Denied,
    /// No response within timeout period
    Timeout,
}

/// Execution result from a state
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether command executed successfully
    pub success: bool,
    /// Command exit code
    pub exit_code: Option<i32>,
    /// Execution duration in milliseconds
    pub execution_duration_ms: u64,
    /// Determined next state
    pub next_state: String,
    /// Reason for transition
    pub transition_type: TransitionType,
    /// Which retry attempt this was
    pub retry_attempt: Option<u32>,
    /// Error description if failed
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status_serialization() {
        let status = ExecutionStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");
    }

    #[test]
    fn test_workflow_definition_creation() {
        let mut states = HashMap::new();
        states.insert(
            "start".to_string(),
            WorkflowState {
                name: "start".to_string(),
                description: "Initial state".to_string(),
                command: Some("echo hello".to_string()),
                timeout_seconds: Some(30),
                requires_approval: false,
                approval_timeout: None,
                approval_description: None,
                retry_policy: None,
                transitions: Some(TransitionRules {
                    success: Some("end".to_string()),
                    failure: None,
                    timeout: None,
                    approval_denied: None,
                }),
                timeout_behavior: TimeoutBehavior::DenyAndFail,
            },
        );

        let workflow = WorkflowDefinition {
            name: "test-workflow".to_string(),
            description: Some("Test workflow".to_string()),
            initial_state: "start".to_string(),
            terminal_states: vec!["end".to_string()],
            states,
            defaults: None,
        };

        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.initial_state, "start");
        assert_eq!(workflow.terminal_states.len(), 1);
    }

    #[test]
    fn test_retry_policy_defaults() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_seconds: 5,
            exponential_backoff: true,
            backoff_multiplier: default_backoff_multiplier(),
        };

        assert_eq!(policy.backoff_multiplier, 2.0);
    }
}
