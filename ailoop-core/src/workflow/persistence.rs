//! Workflow state persistence using JSON file storage

use crate::models::workflow::{
    ApprovalRequest, ApprovalStatus, ExecutionOutput, ExecutionStatus, StateTransition,
    WorkflowExecutionInstance,
};
use anyhow::{Context, Result};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Root JSON store containing all workflow data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonStore {
    /// All workflow execution instances
    pub executions: Vec<WorkflowExecutionInstance>,
    /// All state transitions
    pub transitions: Vec<StateTransition>,
    /// All output chunks
    pub output: Vec<ExecutionOutput>,
    /// All approval requests
    pub approvals: Vec<ApprovalRequest>,
}

/// Workflow persistence manager
pub struct WorkflowPersistence {
    /// Path to JSON store file
    store_path: PathBuf,
    /// In-memory data store
    store: Arc<Mutex<JsonStore>>,
}

impl WorkflowPersistence {
    /// Create new persistence manager
    pub fn new<P: AsRef<Path>>(store_path: P) -> Result<Self> {
        let store_path = store_path.as_ref().to_path_buf();

        // Create parent directory if it doesn't exist
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create workflow store directory")?;
        }

        // Load or initialize store
        let store = if store_path.exists() {
            Self::load_store(&store_path)?
        } else {
            JsonStore::default()
        };

        Ok(Self {
            store_path,
            store: Arc::new(Mutex::new(store)),
        })
    }

    /// Load JSON store from file with file locking
    fn load_store(path: &Path) -> Result<JsonStore> {
        let file = File::open(path).context("Failed to open workflow store file")?;

        // Acquire shared lock for reading
        file.lock_shared()
            .context("Failed to acquire read lock on workflow store")?;

        let mut contents = String::new();
        let mut reader = std::io::BufReader::new(file);
        reader
            .read_to_string(&mut contents)
            .context("Failed to read workflow store")?;

        // Release lock automatically when file goes out of scope
        drop(reader);

        if contents.is_empty() {
            return Ok(JsonStore::default());
        }

        serde_json::from_str(&contents).context("Failed to parse workflow store JSON")
    }

    /// Save JSON store to file with file locking
    fn save_store(&self) -> Result<()> {
        let store = self.store.lock().unwrap();

        // Open file with write permissions
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.store_path)
            .context("Failed to open workflow store file for writing")?;

        // Acquire exclusive lock for writing
        file.lock_exclusive()
            .context("Failed to acquire write lock on workflow store")?;

        let json =
            serde_json::to_string_pretty(&*store).context("Failed to serialize workflow store")?;

        let mut writer = std::io::BufWriter::new(file);
        writer
            .write_all(json.as_bytes())
            .context("Failed to write workflow store")?;

        writer
            .flush()
            .context("Failed to flush workflow store to disk")?;

        // Lock released automatically when writer/file goes out of scope
        Ok(())
    }

    /// Create new workflow execution
    pub fn create_execution(&self, execution: WorkflowExecutionInstance) -> Result<()> {
        {
            let mut store = self.store.lock().unwrap();
            store.executions.push(execution);
        }
        self.save_store()
    }

    /// Update execution status
    pub fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
        current_state: Option<String>,
    ) -> Result<()> {
        {
            let mut store = self.store.lock().unwrap();
            if let Some(execution) = store.executions.iter_mut().find(|e| e.id == execution_id) {
                execution.status = status.clone();
                if let Some(state) = current_state {
                    execution.current_state = state;
                }
                // Set completed_at for terminal states
                if matches!(
                    status,
                    ExecutionStatus::Completed
                        | ExecutionStatus::Failed
                        | ExecutionStatus::Timeout
                        | ExecutionStatus::Denied
                        | ExecutionStatus::Cancelled
                ) {
                    execution.completed_at = Some(Utc::now());
                }
            }
        }
        self.save_store()
    }

    /// Get execution by ID
    pub fn get_execution(&self, execution_id: Uuid) -> Option<WorkflowExecutionInstance> {
        let store = self.store.lock().unwrap();
        store
            .executions
            .iter()
            .find(|e| e.id == execution_id)
            .cloned()
    }

    /// Find incomplete executions for crash recovery
    pub fn find_incomplete_executions(&self) -> Vec<WorkflowExecutionInstance> {
        let store = self.store.lock().unwrap();
        store
            .executions
            .iter()
            .filter(|e| {
                matches!(
                    e.status,
                    ExecutionStatus::Running | ExecutionStatus::ApprovalPending
                )
            })
            .cloned()
            .collect()
    }

    /// Persist state transition
    pub fn persist_state_transition(&self, transition: StateTransition) -> Result<()> {
        {
            let mut store = self.store.lock().unwrap();
            store.transitions.push(transition);
        }
        self.save_store()
    }

    /// Get transitions for execution
    pub fn get_transitions(&self, execution_id: Uuid) -> Vec<StateTransition> {
        let store = self.store.lock().unwrap();
        store
            .transitions
            .iter()
            .filter(|t| t.execution_id == execution_id)
            .cloned()
            .collect()
    }

    /// Create approval request
    pub fn create_approval_request(&self, request: ApprovalRequest) -> Result<()> {
        {
            let mut store = self.store.lock().unwrap();
            store.approvals.push(request);
        }
        self.save_store()
    }

    /// Update approval status
    pub fn update_approval_status(
        &self,
        approval_id: Uuid,
        status: ApprovalStatus,
        responder: Option<String>,
    ) -> Result<()> {
        {
            let mut store = self.store.lock().unwrap();
            if let Some(approval) = store.approvals.iter_mut().find(|a| a.id == approval_id) {
                approval.status = status;
                approval.responded_at = Some(Utc::now());
                approval.responder = responder;
            }
        }
        self.save_store()
    }

    /// Get approval request by ID
    pub fn get_approval_request(&self, approval_id: Uuid) -> Option<ApprovalRequest> {
        let store = self.store.lock().unwrap();
        store
            .approvals
            .iter()
            .find(|a| a.id == approval_id)
            .cloned()
    }

    /// Get pending approvals for execution
    pub fn get_pending_approvals(&self, execution_id: Uuid) -> Vec<ApprovalRequest> {
        let store = self.store.lock().unwrap();
        store
            .approvals
            .iter()
            .filter(|a| a.execution_id == execution_id && a.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    /// Persist output batch
    pub fn persist_output_batch(&self, outputs: Vec<ExecutionOutput>) -> Result<()> {
        {
            let mut store = self.store.lock().unwrap();
            store.output.extend(outputs);
        }
        self.save_store()
    }

    /// Query output for execution and state
    pub fn query_output(
        &self,
        execution_id: Uuid,
        state_name: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Vec<ExecutionOutput> {
        let store = self.store.lock().unwrap();
        let mut outputs: Vec<_> = store
            .output
            .iter()
            .filter(|o| {
                o.execution_id == execution_id && state_name.is_none_or(|s| o.state_name == s)
            })
            .cloned()
            .collect();

        // Sort by sequence number
        outputs.sort_by_key(|o| o.chunk_sequence);

        outputs.into_iter().skip(offset).take(limit).collect()
    }

    /// Query metrics for workflow
    pub fn query_metrics(&self, workflow_name: Option<&str>) -> WorkflowMetrics {
        let store = self.store.lock().unwrap();

        let executions: Vec<_> = store
            .executions
            .iter()
            .filter(|e| workflow_name.is_none_or(|w| e.workflow_name == w))
            .collect();

        let total = executions.len();
        let success = executions
            .iter()
            .filter(|e| e.status == ExecutionStatus::Completed)
            .count();
        let failed = executions
            .iter()
            .filter(|e| {
                matches!(
                    e.status,
                    ExecutionStatus::Failed
                        | ExecutionStatus::Timeout
                        | ExecutionStatus::Denied
                        | ExecutionStatus::Cancelled
                )
            })
            .count();

        let durations: Vec<_> = executions
            .iter()
            .filter_map(|e| {
                e.completed_at
                    .map(|completed| (completed - e.started_at).num_milliseconds() as u64)
            })
            .collect();

        let avg_duration_ms = if !durations.is_empty() {
            durations.iter().sum::<u64>() / durations.len() as u64
        } else {
            0
        };

        WorkflowMetrics {
            execution_count: total,
            success_count: success,
            failure_count: failed,
            avg_duration_ms,
        }
    }
}

/// Workflow execution metrics
#[derive(Debug, Clone)]
pub struct WorkflowMetrics {
    pub execution_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub avg_duration_ms: u64,
}

impl WorkflowMetrics {
    /// Calculate failure rate (FR-039, T078)
    /// Returns the percentage of failed executions (0.0 to 100.0)
    /// Includes timeout and cancelled executions as failures
    pub fn failure_rate(&self) -> f64 {
        if self.execution_count == 0 {
            return 0.0;
        }
        (self.failure_count as f64 / self.execution_count as f64) * 100.0
    }

    /// Calculate success rate
    /// Returns the percentage of successful executions (0.0 to 100.0)
    pub fn success_rate(&self) -> f64 {
        if self.execution_count == 0 {
            return 0.0;
        }
        (self.success_count as f64 / self.execution_count as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_persistence_initialization() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");

        let persistence = WorkflowPersistence::new(&store_path).unwrap();

        // File may not exist until first save, but parent directory should exist
        assert!(store_path.parent().unwrap().exists());

        // Verify empty store
        let store = persistence.store.lock().unwrap();
        assert_eq!(store.executions.len(), 0);
    }

    #[test]
    fn test_create_and_get_execution() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");
        let persistence = WorkflowPersistence::new(&store_path).unwrap();

        let execution = WorkflowExecutionInstance {
            id: Uuid::new_v4(),
            workflow_name: "test-workflow".to_string(),
            current_state: "start".to_string(),
            status: ExecutionStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            initiator: "test-user".to_string(),
            context: None,
        };

        let execution_id = execution.id;
        persistence.create_execution(execution).unwrap();

        let retrieved = persistence.get_execution(execution_id).unwrap();
        assert_eq!(retrieved.workflow_name, "test-workflow");
        assert_eq!(retrieved.status, ExecutionStatus::Running);
    }
}
