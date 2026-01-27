//! Unit tests for workflow approval mechanism

use ailoop_core::models::workflow::{ApprovalRequest, ApprovalStatus, TimeoutBehavior};
use ailoop_core::workflow::WorkflowPersistence;
use tempfile::tempdir;
use uuid::Uuid;

/// Test creating an approval request
#[tokio::test]
async fn test_create_approval_request() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("workflow.json");
    let persistence = WorkflowPersistence::new(&store_path).unwrap();

    let execution_id = Uuid::new_v4();
    let approval = ApprovalRequest {
        id: Uuid::new_v4(),
        execution_id,
        state_name: "production-deploy".to_string(),
        action_description: "Deploy to production environment".to_string(),
        status: ApprovalStatus::Pending,
        requested_at: chrono::Utc::now(),
        responded_at: None,
        responder: None,
        timeout_seconds: 300,
        timeout_behavior: TimeoutBehavior::DenyAndFail,
        context: None,
    };

    let approval_id = approval.id;
    persistence.create_approval_request(approval).unwrap();

    // Verify approval was created
    let retrieved = persistence.get_approval_request(approval_id).unwrap();
    assert_eq!(retrieved.status, ApprovalStatus::Pending);
    assert_eq!(retrieved.state_name, "production-deploy");
}

/// Test approving an approval request
#[tokio::test]
async fn test_approve_approval_request() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("workflow.json");
    let persistence = WorkflowPersistence::new(&store_path).unwrap();

    let approval = ApprovalRequest {
        id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        state_name: "deploy".to_string(),
        action_description: "Deploy application".to_string(),
        status: ApprovalStatus::Pending,
        requested_at: chrono::Utc::now(),
        responded_at: None,
        responder: None,
        timeout_seconds: 300,
        timeout_behavior: TimeoutBehavior::DenyAndFail,
        context: None,
    };

    let approval_id = approval.id;
    persistence.create_approval_request(approval).unwrap();

    // Approve the request
    persistence
        .update_approval_status(
            approval_id,
            ApprovalStatus::Approved,
            Some("operator1".to_string()),
        )
        .unwrap();

    // Verify status updated
    let retrieved = persistence.get_approval_request(approval_id).unwrap();
    assert_eq!(retrieved.status, ApprovalStatus::Approved);
    assert_eq!(retrieved.responder, Some("operator1".to_string()));
    assert!(retrieved.responded_at.is_some());
}

/// Test denying an approval request
#[tokio::test]
async fn test_deny_approval_request() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("workflow.json");
    let persistence = WorkflowPersistence::new(&store_path).unwrap();

    let approval = ApprovalRequest {
        id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        state_name: "deploy".to_string(),
        action_description: "Deploy application".to_string(),
        status: ApprovalStatus::Pending,
        requested_at: chrono::Utc::now(),
        responded_at: None,
        responder: None,
        timeout_seconds: 300,
        timeout_behavior: TimeoutBehavior::DenyAndFail,
        context: None,
    };

    let approval_id = approval.id;
    persistence.create_approval_request(approval).unwrap();

    // Deny the request
    persistence
        .update_approval_status(
            approval_id,
            ApprovalStatus::Denied,
            Some("operator2".to_string()),
        )
        .unwrap();

    // Verify status updated
    let retrieved = persistence.get_approval_request(approval_id).unwrap();
    assert_eq!(retrieved.status, ApprovalStatus::Denied);
    assert_eq!(retrieved.responder, Some("operator2".to_string()));
}

/// Test querying pending approvals
#[tokio::test]
async fn test_get_pending_approvals() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("workflow.json");
    let persistence = WorkflowPersistence::new(&store_path).unwrap();

    let execution_id = Uuid::new_v4();

    // Create multiple approval requests
    for i in 0..3 {
        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            execution_id,
            state_name: format!("step-{}", i),
            action_description: format!("Action {}", i),
            status: if i == 1 {
                ApprovalStatus::Approved
            } else {
                ApprovalStatus::Pending
            },
            requested_at: chrono::Utc::now(),
            responded_at: None,
            responder: None,
            timeout_seconds: 300,
            timeout_behavior: TimeoutBehavior::DenyAndFail,
            context: None,
        };
        persistence.create_approval_request(approval).unwrap();
    }

    // Query pending approvals
    let pending = persistence.get_pending_approvals(execution_id);
    assert_eq!(pending.len(), 2); // Only pending ones
}

/// Test approval timeout behavior
#[tokio::test]
async fn test_approval_timeout() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("workflow.json");
    let persistence = WorkflowPersistence::new(&store_path).unwrap();

    let approval = ApprovalRequest {
        id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        state_name: "deploy".to_string(),
        action_description: "Deploy application".to_string(),
        status: ApprovalStatus::Pending,
        requested_at: chrono::Utc::now(),
        responded_at: None,
        responder: None,
        timeout_seconds: 1, // Very short timeout
        timeout_behavior: TimeoutBehavior::DenyAndFail,
        context: None,
    };

    let approval_id = approval.id;
    persistence.create_approval_request(approval).unwrap();

    // Simulate timeout
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Mark as timed out
    persistence
        .update_approval_status(approval_id, ApprovalStatus::Timeout, None)
        .unwrap();

    // Verify timeout status
    let retrieved = persistence.get_approval_request(approval_id).unwrap();
    assert_eq!(retrieved.status, ApprovalStatus::Timeout);
}
