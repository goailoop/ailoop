//! Workflow orchestration and lifecycle management

use crate::models::workflow::{ExecutionStatus, WorkflowDefinition, WorkflowExecutionInstance};
use crate::workflow::approval_manager::ApprovalManager;
use crate::workflow::engine::StateMachineEngine;
use crate::workflow::executor::StateMachineExecutor;
use crate::workflow::persistence::WorkflowPersistence;
use anyhow::{Context, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Workflow orchestrator managing active workflow executions
pub struct WorkflowOrchestrator {
    /// Active workflow execution handles
    active_workflows: Arc<DashMap<Uuid, JoinHandle<Result<ExecutionStatus>>>>,
    /// Workflow definitions registry
    workflow_definitions: Arc<DashMap<String, WorkflowDefinition>>,
    /// Persistence layer
    persistence: Arc<WorkflowPersistence>,
    /// State executor
    executor: Arc<dyn StateMachineExecutor>,
    /// Approval manager
    approval_manager: Arc<ApprovalManager>,
}

impl WorkflowOrchestrator {
    /// Create new workflow orchestrator
    pub fn new(
        persistence: Arc<WorkflowPersistence>,
        executor: Arc<dyn StateMachineExecutor>,
    ) -> Self {
        let approval_manager = Arc::new(ApprovalManager::new(persistence.clone()));

        Self {
            active_workflows: Arc::new(DashMap::new()),
            workflow_definitions: Arc::new(DashMap::new()),
            persistence,
            executor,
            approval_manager,
        }
    }

    /// Get approval manager reference
    pub fn approval_manager(&self) -> Arc<ApprovalManager> {
        self.approval_manager.clone()
    }

    /// Register a workflow definition
    pub fn register_workflow(&self, workflow_def: WorkflowDefinition) {
        let name = workflow_def.name.clone();
        self.workflow_definitions.insert(name, workflow_def);
    }

    /// Start a workflow execution
    pub async fn start_workflow(&self, workflow_name: &str, initiator: String) -> Result<Uuid> {
        // Get workflow definition
        let workflow_def = self
            .workflow_definitions
            .get(workflow_name)
            .ok_or_else(|| anyhow::anyhow!("Workflow '{}' not found", workflow_name))?
            .clone();

        // Generate execution ID
        let execution_id = Uuid::new_v4();

        // Create engine
        let engine = StateMachineEngine::new(
            workflow_def,
            self.executor.clone(),
            self.persistence.clone(),
            self.approval_manager.clone(),
        );

        // Spawn async task for workflow execution
        let execution_id_clone = execution_id;
        let handle = tokio::spawn(async move {
            tracing::info!("Starting workflow execution {}", execution_id_clone);
            let result = engine.execute(execution_id_clone, initiator).await;
            tracing::info!(
                "Workflow execution {} completed: {:?}",
                execution_id_clone,
                result
            );
            result
        });

        // Track active workflow
        self.active_workflows.insert(execution_id, handle);

        tracing::info!(
            "Started workflow '{}' with execution ID {}",
            workflow_name,
            execution_id
        );

        Ok(execution_id)
    }

    /// Get workflow execution status
    pub fn get_execution_status(&self, execution_id: Uuid) -> Option<WorkflowExecutionInstance> {
        self.persistence.get_execution(execution_id)
    }

    /// Check if workflow is currently running
    pub fn is_running(&self, execution_id: Uuid) -> bool {
        self.active_workflows.contains_key(&execution_id)
    }

    /// Get count of active workflows
    pub fn active_count(&self) -> usize {
        self.active_workflows.len()
    }

    /// Cancel a running workflow
    pub async fn cancel_workflow(&self, execution_id: Uuid) -> Result<()> {
        if let Some((_key, handle)) = self.active_workflows.remove(&execution_id) {
            handle.abort();

            // Update execution status to cancelled
            self.persistence
                .update_execution_status(execution_id, ExecutionStatus::Cancelled, None)
                .context("Failed to update execution status to cancelled")?;

            tracing::info!("Cancelled workflow execution {}", execution_id);
        }

        Ok(())
    }

    /// Wait for workflow to complete
    pub async fn wait_for_completion(&self, execution_id: Uuid) -> Result<ExecutionStatus> {
        if let Some((_key, handle)) = self.active_workflows.remove(&execution_id) {
            match handle.await {
                Ok(result) => result,
                Err(e) if e.is_cancelled() => Ok(ExecutionStatus::Cancelled),
                Err(e) => Err(anyhow::anyhow!("Workflow execution task panicked: {}", e)),
            }
        } else {
            // Workflow not in active map, check persistence
            let execution = self
                .persistence
                .get_execution(execution_id)
                .ok_or_else(|| anyhow::anyhow!("Execution {} not found", execution_id))?;

            Ok(execution.status)
        }
    }

    /// List all registered workflow definitions
    pub fn list_workflows(&self) -> Vec<String> {
        self.workflow_definitions
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get workflow definition
    pub fn get_workflow_definition(&self, name: &str) -> Option<WorkflowDefinition> {
        self.workflow_definitions
            .get(name)
            .map(|entry| entry.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::workflow::{TransitionRules, WorkflowState};
    use crate::workflow::bash_executor::BashExecutor;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_orchestrator_start_workflow() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");

        let persistence = Arc::new(WorkflowPersistence::new(&store_path).unwrap());
        let executor = Arc::new(BashExecutor::new());

        let orchestrator = WorkflowOrchestrator::new(persistence.clone(), executor);

        // Register a simple workflow
        let mut states = HashMap::new();
        states.insert(
            "start".to_string(),
            WorkflowState {
                name: "start".to_string(),
                description: "Start state".to_string(),
                command: Some("echo 'hello'".to_string()),
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

        orchestrator.register_workflow(workflow_def);

        // Start workflow
        let execution_id = orchestrator
            .start_workflow("test-workflow", "test-user".to_string())
            .await
            .unwrap();

        assert!(orchestrator.is_running(execution_id));

        // Wait for completion
        let final_status = orchestrator
            .wait_for_completion(execution_id)
            .await
            .unwrap();
        assert_eq!(final_status, ExecutionStatus::Completed);

        // Verify no longer in active map
        assert!(!orchestrator.is_running(execution_id));
    }

    #[tokio::test]
    async fn test_orchestrator_list_workflows() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");

        let persistence = Arc::new(WorkflowPersistence::new(&store_path).unwrap());
        let executor = Arc::new(BashExecutor::new());

        let orchestrator = WorkflowOrchestrator::new(persistence, executor);

        // Register multiple workflows
        for i in 1..=3 {
            let workflow_def = WorkflowDefinition {
                name: format!("workflow-{}", i),
                description: Some(format!("Test workflow {}", i)),
                initial_state: "start".to_string(),
                terminal_states: vec!["completed".to_string()],
                states: HashMap::new(),
                defaults: None,
            };
            orchestrator.register_workflow(workflow_def);
        }

        let workflows = orchestrator.list_workflows();
        assert_eq!(workflows.len(), 3);
        assert!(workflows.contains(&"workflow-1".to_string()));
        assert!(workflows.contains(&"workflow-2".to_string()));
        assert!(workflows.contains(&"workflow-3".to_string()));
    }
}
