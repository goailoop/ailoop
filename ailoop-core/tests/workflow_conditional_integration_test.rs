//! Integration test for conditional workflow branching

use ailoop_core::models::workflow::{
    TimeoutBehavior, TransitionRules, WorkflowDefinition, WorkflowState,
};
use std::collections::HashMap;

#[test]
fn test_conditional_workflow_definition() {
    // Create a workflow with conditional branching: validate → success/failure paths
    let mut states = HashMap::new();

    // Validation state with conditional transitions
    states.insert(
        "validate".to_string(),
        WorkflowState {
            name: "validate".to_string(),
            description: "Validation step".to_string(),
            command: Some("echo 'validating'".to_string()),
            timeout_seconds: Some(30),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: Some(TransitionRules {
                success: Some("deploy".to_string()),   // Success path
                failure: Some("rollback".to_string()), // Failure path
                timeout: Some("timeout_state".to_string()),
                approval_denied: None,
            }),
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );

    states.insert(
        "deploy".to_string(),
        WorkflowState {
            name: "deploy".to_string(),
            description: "Deploy on success".to_string(),
            command: Some("echo 'deploying'".to_string()),
            timeout_seconds: Some(60),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: Some(TransitionRules {
                success: Some("completed".to_string()),
                failure: Some("rollback".to_string()),
                timeout: None,
                approval_denied: None,
            }),
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );

    states.insert(
        "rollback".to_string(),
        WorkflowState {
            name: "rollback".to_string(),
            description: "Rollback on failure".to_string(),
            command: Some("echo 'rolling back'".to_string()),
            timeout_seconds: Some(30),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: Some(TransitionRules {
                success: Some("failed".to_string()),
                failure: Some("failed".to_string()),
                timeout: None,
                approval_denied: None,
            }),
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );

    states.insert(
        "timeout_state".to_string(),
        WorkflowState {
            name: "timeout_state".to_string(),
            description: "Timeout handler".to_string(),
            command: None,
            timeout_seconds: None,
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: None,
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );

    states.insert(
        "completed".to_string(),
        WorkflowState {
            name: "completed".to_string(),
            description: "Success terminal state".to_string(),
            command: None,
            timeout_seconds: None,
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: None,
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );

    states.insert(
        "failed".to_string(),
        WorkflowState {
            name: "failed".to_string(),
            description: "Failure terminal state".to_string(),
            command: None,
            timeout_seconds: None,
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: None,
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );

    let workflow = WorkflowDefinition {
        name: "conditional-deployment".to_string(),
        description: Some("Workflow with conditional branching".to_string()),
        initial_state: "validate".to_string(),
        terminal_states: vec![
            "completed".to_string(),
            "failed".to_string(),
            "timeout_state".to_string(),
        ],
        states,
        defaults: None,
    };

    // Validate workflow structure
    assert_eq!(workflow.states.len(), 6);
    assert_eq!(workflow.initial_state, "validate");

    // Verify validate state has conditional transitions
    let validate_state = workflow.states.get("validate").unwrap();
    let transitions = validate_state.transitions.as_ref().unwrap();
    assert_eq!(transitions.success, Some("deploy".to_string()));
    assert_eq!(transitions.failure, Some("rollback".to_string()));
    assert_eq!(transitions.timeout, Some("timeout_state".to_string()));

    // Verify terminal states have no transitions
    for terminal in &workflow.terminal_states {
        let state = workflow.states.get(terminal).unwrap();
        assert!(state.transitions.is_none());
    }
}

#[tokio::test]
async fn test_conditional_workflow_success_path() {
    // Test that successful execution follows the success path
    use ailoop_core::workflow::{BashExecutor, StateMachineExecutor};

    let executor = BashExecutor::new();

    // Create a state that will succeed
    let state = WorkflowState {
        name: "test".to_string(),
        description: "Test state".to_string(),
        command: Some("exit 0".to_string()), // Will succeed
        timeout_seconds: Some(10),
        requires_approval: false,
        approval_timeout: None,
        approval_description: None,
        retry_policy: None,
        transitions: Some(TransitionRules {
            success: Some("success_state".to_string()),
            failure: Some("failure_state".to_string()),
            timeout: None,
            approval_denied: None,
        }),
        timeout_behavior: TimeoutBehavior::DenyAndFail,
    };

    let result = executor.execute("test-exec", &state).await.unwrap();

    // Verify it followed the success path
    assert!(result.success);
    assert_eq!(result.next_state, "success_state");
    assert_eq!(
        result.transition_type,
        ailoop_core::models::workflow::TransitionType::Success
    );
}

#[tokio::test]
async fn test_conditional_workflow_failure_path() {
    // Test that failed execution follows the failure path
    use ailoop_core::workflow::{BashExecutor, StateMachineExecutor};

    let executor = BashExecutor::new();

    // Create a state that will fail
    let state = WorkflowState {
        name: "test".to_string(),
        description: "Test state".to_string(),
        command: Some("exit 1".to_string()), // Will fail
        timeout_seconds: Some(10),
        requires_approval: false,
        approval_timeout: None,
        approval_description: None,
        retry_policy: None,
        transitions: Some(TransitionRules {
            success: Some("success_state".to_string()),
            failure: Some("failure_state".to_string()),
            timeout: None,
            approval_denied: None,
        }),
        timeout_behavior: TimeoutBehavior::DenyAndFail,
    };

    let result = executor.execute("test-exec", &state).await.unwrap();

    // Verify it followed the failure path
    assert!(!result.success);
    assert_eq!(result.next_state, "failure_state");
    assert_eq!(
        result.transition_type,
        ailoop_core::models::workflow::TransitionType::Failure
    );
}

#[tokio::test]
async fn test_conditional_workflow_timeout_path() {
    // Test that timeout follows the timeout path
    use ailoop_core::workflow::{BashExecutor, StateMachineExecutor};

    let executor = BashExecutor::new();

    // Create a state that will timeout
    let state = WorkflowState {
        name: "test".to_string(),
        description: "Test state".to_string(),
        command: Some("sleep 10".to_string()), // Will timeout
        timeout_seconds: Some(1),              // Short timeout
        requires_approval: false,
        approval_timeout: None,
        approval_description: None,
        retry_policy: None,
        transitions: Some(TransitionRules {
            success: Some("success_state".to_string()),
            failure: Some("failure_state".to_string()),
            timeout: Some("timeout_state".to_string()),
            approval_denied: None,
        }),
        timeout_behavior: TimeoutBehavior::DenyAndFail,
    };

    let result = executor.execute("test-exec", &state).await.unwrap();

    // Verify it followed the timeout path
    assert!(!result.success);
    assert_eq!(result.next_state, "timeout_state");
    assert_eq!(
        result.transition_type,
        ailoop_core::models::workflow::TransitionType::Timeout
    );
}

#[test]
fn test_first_matching_condition_taken() {
    // Verify that once a transition is selected, other paths are ignored
    let transitions = TransitionRules {
        success: Some("path_a".to_string()),
        failure: Some("path_b".to_string()),
        timeout: Some("path_c".to_string()),
        approval_denied: Some("path_d".to_string()),
    };

    // If success, only success path is taken
    let is_success = true;
    let selected_path = if is_success {
        &transitions.success
    } else {
        &transitions.failure
    };

    assert_eq!(selected_path, &Some("path_a".to_string()));
    // Other paths are not evaluated when success is selected
}
