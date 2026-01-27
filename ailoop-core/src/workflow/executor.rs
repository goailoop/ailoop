//! State machine executor trait and types

use crate::models::workflow::{ExecutionResult, WorkflowState};
use anyhow::Result;
use async_trait::async_trait;

/// Trait for executing workflow states
#[async_trait]
pub trait StateMachineExecutor: Send + Sync {
    /// Execute a workflow state
    ///
    /// # Arguments
    /// * `execution_id` - Unique identifier for the workflow execution
    /// * `state` - State definition to execute
    ///
    /// # Returns
    /// Execution result with next state determination
    async fn execute(&self, execution_id: &str, state: &WorkflowState) -> Result<ExecutionResult>;
}
