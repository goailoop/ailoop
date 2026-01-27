//! Workflow approval request management

use crate::models::workflow::{ApprovalRequest, ApprovalStatus, TimeoutBehavior};
use crate::workflow::persistence::WorkflowPersistence;
use anyhow::{Context, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;

/// Response from an approval request
#[derive(Debug, Clone)]
pub enum ApprovalResponse {
    /// Approval was granted
    Approved,
    /// Approval was denied
    Denied,
    /// Request timed out
    Timeout,
}

/// Manages workflow approval requests and responses
pub struct ApprovalManager {
    /// Persistence layer
    persistence: Arc<WorkflowPersistence>,
    /// Pending approval response channels
    pending_channels: Arc<DashMap<Uuid, oneshot::Sender<ApprovalResponse>>>,
}

impl ApprovalManager {
    /// Create new approval manager
    pub fn new(persistence: Arc<WorkflowPersistence>) -> Self {
        Self {
            persistence,
            pending_channels: Arc::new(DashMap::new()),
        }
    }

    /// Create and send an approval request
    pub async fn request_approval(
        &self,
        execution_id: Uuid,
        state_name: String,
        action_description: String,
        timeout_seconds: u32,
        timeout_behavior: TimeoutBehavior,
    ) -> Result<(Uuid, oneshot::Receiver<ApprovalResponse>)> {
        // Create approval request
        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            execution_id,
            state_name,
            action_description,
            status: ApprovalStatus::Pending,
            requested_at: chrono::Utc::now(),
            responded_at: None,
            responder: None,
            timeout_seconds,
            timeout_behavior,
            context: None,
        };

        let approval_id = approval.id;

        // Persist approval request
        self.persistence
            .create_approval_request(approval)
            .context("Failed to persist approval request")?;

        // Create response channel
        let (tx, rx) = oneshot::channel();
        self.pending_channels.insert(approval_id, tx);

        tracing::info!(
            "Approval request {} created for execution {}",
            approval_id,
            execution_id
        );

        Ok((approval_id, rx))
    }

    /// Send approval response
    pub async fn respond_approval(
        &self,
        approval_id: Uuid,
        response: ApprovalResponse,
        responder: Option<String>,
    ) -> Result<()> {
        // Update persistence
        let status = match response {
            ApprovalResponse::Approved => ApprovalStatus::Approved,
            ApprovalResponse::Denied => ApprovalStatus::Denied,
            ApprovalResponse::Timeout => ApprovalStatus::Timeout,
        };

        self.persistence
            .update_approval_status(approval_id, status, responder.clone())
            .context("Failed to update approval status")?;

        // Send response through channel if still pending
        if let Some((_key, tx)) = self.pending_channels.remove(&approval_id) {
            let _ = tx.send(response.clone());
            tracing::info!("Approval response sent for {}: {:?}", approval_id, response);
        }

        Ok(())
    }

    /// Get approval request by ID
    pub fn get_approval_request(&self, approval_id: Uuid) -> Option<ApprovalRequest> {
        self.persistence.get_approval_request(approval_id)
    }

    /// Get pending approvals for execution
    pub fn get_pending_approvals(&self, execution_id: Uuid) -> Vec<ApprovalRequest> {
        self.persistence.get_pending_approvals(execution_id)
    }

    /// List all pending approval requests
    pub fn list_all_pending(&self) -> Vec<ApprovalRequest> {
        // This requires iterating through all approvals in persistence
        // For now, return empty vec - will be implemented with proper persistence query
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_approval_request_and_response() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");
        let persistence = Arc::new(WorkflowPersistence::new(&store_path).unwrap());

        let manager = ApprovalManager::new(persistence.clone());

        // Request approval
        let execution_id = Uuid::new_v4();
        let (approval_id, rx) = manager
            .request_approval(
                execution_id,
                "deploy".to_string(),
                "Deploy to production".to_string(),
                300,
                TimeoutBehavior::DenyAndFail,
            )
            .await
            .unwrap();

        // Verify request was persisted
        let approval = manager.get_approval_request(approval_id).unwrap();
        assert_eq!(approval.status, ApprovalStatus::Pending);

        // Send approval response
        manager
            .respond_approval(
                approval_id,
                ApprovalResponse::Approved,
                Some("operator1".to_string()),
            )
            .await
            .unwrap();

        // Verify channel received response
        let response = rx.await.unwrap();
        matches!(response, ApprovalResponse::Approved);

        // Verify status was updated in persistence
        let updated_approval = manager.get_approval_request(approval_id).unwrap();
        assert_eq!(updated_approval.status, ApprovalStatus::Approved);
        assert_eq!(updated_approval.responder, Some("operator1".to_string()));
    }

    #[tokio::test]
    async fn test_approval_denial() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("workflow.json");
        let persistence = Arc::new(WorkflowPersistence::new(&store_path).unwrap());

        let manager = ApprovalManager::new(persistence);

        let execution_id = Uuid::new_v4();
        let (approval_id, rx) = manager
            .request_approval(
                execution_id,
                "deploy".to_string(),
                "Deploy to production".to_string(),
                300,
                TimeoutBehavior::DenyAndFail,
            )
            .await
            .unwrap();

        // Deny the request
        manager
            .respond_approval(
                approval_id,
                ApprovalResponse::Denied,
                Some("operator2".to_string()),
            )
            .await
            .unwrap();

        let response = rx.await.unwrap();
        matches!(response, ApprovalResponse::Denied);

        let approval = manager.get_approval_request(approval_id).unwrap();
        assert_eq!(approval.status, ApprovalStatus::Denied);
    }
}
