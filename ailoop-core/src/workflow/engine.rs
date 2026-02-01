//! State machine execution engine

use crate::models::workflow::{
    ExecutionStatus, StateTransition, TimeoutBehavior, TransitionType, WorkflowDefinition,
    WorkflowExecutionInstance,
};
use crate::workflow::approval_manager::{ApprovalManager, ApprovalResponse};
use crate::workflow::executor::StateMachineExecutor;
use crate::workflow::persistence::WorkflowPersistence;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

/// State machine execution engine
pub struct StateMachineEngine {
    /// Workflow definition
    workflow_def: WorkflowDefinition,
    /// State executor
    executor: Arc<dyn StateMachineExecutor>,
    /// Persistence layer
    persistence: Arc<WorkflowPersistence>,
    /// Approval manager
    approval_manager: Arc<ApprovalManager>,
}

impl StateMachineEngine {
    /// Create new state machine engine
    pub fn new(
        workflow_def: WorkflowDefinition,
        executor: Arc<dyn StateMachineExecutor>,
        persistence: Arc<WorkflowPersistence>,
        approval_manager: Arc<ApprovalManager>,
    ) -> Self {
        Self {
            workflow_def,
            executor,
            persistence,
            approval_manager,
        }
    }

    /// Execute workflow from start to terminal state
    pub async fn execute(&self, execution_id: Uuid, initiator: String) -> Result<ExecutionStatus> {
        // Create execution instance
        let execution = WorkflowExecutionInstance {
            id: execution_id,
            workflow_name: self.workflow_def.name.clone(),
            current_state: self.workflow_def.initial_state.clone(),
            status: ExecutionStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            initiator,
            context: None,
        };

        self.persistence
            .create_execution(execution.clone())
            .context("Failed to create execution")?;

        // Record initial transition
        let initial_transition = StateTransition {
            id: Uuid::new_v4(),
            execution_id,
            from_state: None,
            to_state: self.workflow_def.initial_state.clone(),
            transition_type: TransitionType::Initial,
            timestamp: Utc::now(),
            duration_ms: None,
            exit_code: None,
            metadata: None,
        };

        self.persistence
            .persist_state_transition(initial_transition)
            .context("Failed to persist initial transition")?;

        // Main state machine loop
        let mut current_state = self.workflow_def.initial_state.clone();

        loop {
            // Check if current state is terminal
            if self.workflow_def.terminal_states.contains(&current_state) {
                let final_status =
                    if current_state == "completed" || current_state.contains("success") {
                        ExecutionStatus::Completed
                    } else {
                        ExecutionStatus::Failed
                    };

                self.persistence
                    .update_execution_status(
                        execution_id,
                        final_status.clone(),
                        Some(current_state),
                    )
                    .context("Failed to update final execution status")?;

                return Ok(final_status);
            }

            // Get state definition
            let state_def = self
                .workflow_def
                .states
                .get(&current_state)
                .ok_or_else(|| {
                    anyhow!("State '{}' not found in workflow definition", current_state)
                })?;

            // Check for approval requirement
            if state_def.requires_approval {
                tracing::info!(
                    "State '{}' requires approval - waiting for human approval",
                    current_state
                );

                // Update execution status to approval pending
                self.persistence
                    .update_execution_status(
                        execution_id,
                        ExecutionStatus::ApprovalPending,
                        Some(current_state.clone()),
                    )
                    .context("Failed to update status to approval pending")?;

                // Request approval
                let approval_timeout_seconds = state_def.approval_timeout.unwrap_or(300);
                let approval_description = state_def
                    .approval_description
                    .clone()
                    .unwrap_or_else(|| format!("Approve state: {}", state_def.description));

                let (_approval_id, rx) = self
                    .approval_manager
                    .request_approval(
                        execution_id,
                        current_state.clone(),
                        approval_description,
                        approval_timeout_seconds,
                        state_def.timeout_behavior.clone(),
                    )
                    .await
                    .context("Failed to request approval")?;

                // Wait for approval response with timeout
                let approval_timeout_duration =
                    Duration::from_secs(approval_timeout_seconds as u64);
                let approval_result = timeout(approval_timeout_duration, rx).await;

                // Restore running status
                self.persistence
                    .update_execution_status(
                        execution_id,
                        ExecutionStatus::Running,
                        Some(current_state.clone()),
                    )
                    .context("Failed to restore running status")?;

                match approval_result {
                    Ok(Ok(ApprovalResponse::Approved)) => {
                        tracing::info!("Approval granted for state '{}'", current_state);
                        // Continue with state execution
                    }
                    Ok(Ok(ApprovalResponse::Denied)) => {
                        tracing::warn!("Approval denied for state '{}'", current_state);

                        // Get denial transition
                        if let Some(transitions) = &state_def.transitions {
                            if let Some(denial_state) = &transitions.approval_denied {
                                // Transition to denial state
                                let transition = StateTransition {
                                    id: Uuid::new_v4(),
                                    execution_id,
                                    from_state: Some(current_state.clone()),
                                    to_state: denial_state.clone(),
                                    transition_type: TransitionType::ApprovalDenied,
                                    timestamp: Utc::now(),
                                    duration_ms: None,
                                    exit_code: None,
                                    metadata: None,
                                };

                                self.persistence
                                    .persist_state_transition(transition)
                                    .context("Failed to persist denial transition")?;

                                current_state = denial_state.clone();
                                continue;
                            }
                        }

                        // No denial transition - mark as denied
                        self.persistence
                            .update_execution_status(
                                execution_id,
                                ExecutionStatus::Denied,
                                Some(current_state),
                            )
                            .context("Failed to update status to denied")?;

                        return Ok(ExecutionStatus::Denied);
                    }
                    Ok(Ok(ApprovalResponse::Timeout)) | Err(_) => {
                        tracing::warn!("Approval timeout for state '{}'", current_state);

                        // Execute timeout behavior
                        match state_def.timeout_behavior {
                            TimeoutBehavior::DenyAndFail => {
                                self.persistence
                                    .update_execution_status(
                                        execution_id,
                                        ExecutionStatus::Timeout,
                                        Some(current_state),
                                    )
                                    .context("Failed to update status to timeout")?;

                                return Ok(ExecutionStatus::Timeout);
                            }
                            TimeoutBehavior::DenyAndContinue => {
                                // Treat as denial and continue
                                if let Some(transitions) = &state_def.transitions {
                                    if let Some(denial_state) = &transitions.approval_denied {
                                        current_state = denial_state.clone();
                                        continue;
                                    }
                                }
                                // No denial state - fail
                                self.persistence
                                    .update_execution_status(
                                        execution_id,
                                        ExecutionStatus::Timeout,
                                        Some(current_state),
                                    )
                                    .context("Failed to update status to timeout")?;

                                return Ok(ExecutionStatus::Timeout);
                            }
                        }
                    }
                    Ok(Err(_)) => {
                        return Err(anyhow!("Approval channel closed unexpectedly"));
                    }
                }
            }

            // Execute state
            let state_start = Instant::now();
            let execution_result = self
                .executor
                .execute(&execution_id.to_string(), state_def)
                .await
                .context(format!("Failed to execute state '{}'", current_state))?;

            let duration_ms = state_start.elapsed().as_millis() as u64;

            // Record state transition
            let transition = StateTransition {
                id: Uuid::new_v4(),
                execution_id,
                from_state: Some(current_state.clone()),
                to_state: execution_result.next_state.clone(),
                transition_type: execution_result.transition_type.clone(),
                timestamp: Utc::now(),
                duration_ms: Some(duration_ms),
                exit_code: execution_result.exit_code,
                metadata: execution_result
                    .retry_attempt
                    .map(|attempt| serde_json::json!({ "retry_attempt": attempt })),
            };

            self.persistence
                .persist_state_transition(transition)
                .context("Failed to persist state transition")?;

            // Update execution current state
            let new_status = if self
                .workflow_def
                .terminal_states
                .contains(&execution_result.next_state)
            {
                if execution_result.next_state.contains("fail") {
                    ExecutionStatus::Failed
                } else {
                    ExecutionStatus::Completed
                }
            } else {
                ExecutionStatus::Running
            };

            self.persistence
                .update_execution_status(
                    execution_id,
                    new_status.clone(),
                    Some(execution_result.next_state.clone()),
                )
                .context("Failed to update execution status")?;

            // Move to next state
            current_state = execution_result.next_state;

            // Emit progress update (TODO: implement broadcasting in later phase)
            tracing::info!(
                "Workflow {} transitioned to state '{}'",
                execution_id,
                current_state
            );
        }
    }

    /// Check if state is terminal
    pub fn is_terminal(&self, state: &str) -> bool {
        self.workflow_def
            .terminal_states
            .contains(&state.to_string())
    }
}

#[cfg(test)]
#[cfg_attr(target_os = "windows", allow(unused_imports))]
mod tests {
    use super::*;
    use crate::models::workflow::{TransitionRules, WorkflowState};
    use crate::workflow::bash_executor::BashExecutor;
    use std::collections::HashMap;
    use tempfile::tempdir;

    /// Uses BashExecutor; skip on Windows where bash may be unavailable.
    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn test_state_machine_simple_workflow() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");

        // Create simple 2-step workflow
        let mut states = HashMap::new();
        states.insert(
            "start".to_string(),
            WorkflowState {
                name: "start".to_string(),
                description: "Start state".to_string(),
                command: Some("echo 'starting'".to_string()),
                timeout_seconds: Some(10),
                requires_approval: false,
                approval_timeout: None,
                approval_description: None,
                retry_policy: None,
                transitions: Some(TransitionRules {
                    success: Some("completed".to_string()),
                    failure: Some("failed".to_string()),
                    timeout: None,
                    approval_denied: None,
                }),
                timeout_behavior: crate::models::workflow::TimeoutBehavior::DenyAndFail,
            },
        );

        states.insert(
            "completed".to_string(),
            WorkflowState {
                name: "completed".to_string(),
                description: "Completed state".to_string(),
                command: None,
                timeout_seconds: None,
                requires_approval: false,
                approval_timeout: None,
                approval_description: None,
                retry_policy: None,
                transitions: None,
                timeout_behavior: crate::models::workflow::TimeoutBehavior::DenyAndFail,
            },
        );

        states.insert(
            "failed".to_string(),
            WorkflowState {
                name: "failed".to_string(),
                description: "Failed state".to_string(),
                command: None,
                timeout_seconds: None,
                requires_approval: false,
                approval_timeout: None,
                approval_description: None,
                retry_policy: None,
                transitions: None,
                timeout_behavior: crate::models::workflow::TimeoutBehavior::DenyAndFail,
            },
        );

        let workflow_def = WorkflowDefinition {
            name: "test-workflow".to_string(),
            description: Some("Test workflow".to_string()),
            initial_state: "start".to_string(),
            terminal_states: vec!["completed".to_string(), "failed".to_string()],
            states,
            defaults: None,
        };

        let executor = Arc::new(BashExecutor::new());
        let persistence = Arc::new(WorkflowPersistence::new(&store_path).unwrap());
        let approval_manager = Arc::new(crate::workflow::ApprovalManager::new(persistence.clone()));

        let engine = StateMachineEngine::new(
            workflow_def,
            executor,
            persistence.clone(),
            approval_manager,
        );

        let execution_id = Uuid::new_v4();
        let final_status = engine
            .execute(execution_id, "test-user".to_string())
            .await
            .unwrap();

        assert_eq!(final_status, ExecutionStatus::Completed);

        // Verify execution was persisted
        let execution = persistence.get_execution(execution_id).unwrap();
        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert_eq!(execution.current_state, "completed");

        // Verify transitions were recorded
        let transitions = persistence.get_transitions(execution_id);
        assert!(transitions.len() >= 2); // Initial + at least one state transition
    }
}
