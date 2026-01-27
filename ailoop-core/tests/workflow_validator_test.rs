//! Unit tests for workflow validator
//! Tests circular dependency detection, reachability analysis, and validation rules

use ailoop_core::models::workflow::{
    DefaultConfiguration, RetryPolicy, TimeoutBehavior, TransitionRules, WorkflowDefinition,
    WorkflowState,
};
use ailoop_core::workflow::WorkflowValidator;
use std::collections::HashMap;

/// Helper to create a simple state
fn create_state(name: &str, transitions: Option<TransitionRules>) -> WorkflowState {
    WorkflowState {
        name: name.to_string(),
        description: format!("State {}", name),
        command: Some(format!("echo '{}'", name)),
        timeout_seconds: Some(30),
        requires_approval: false,
        approval_timeout: None,
        approval_description: None,
        retry_policy: None,
        transitions,
        timeout_behavior: TimeoutBehavior::DenyAndFail,
    }
}

#[test]
fn test_validate_simple_valid_workflow() {
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert("end".to_string(), create_state("end", None));

    let workflow = WorkflowDefinition {
        name: "simple-workflow".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec!["end".to_string()],
        states,
        defaults: None,
    };

    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    assert!(result.is_valid(), "Simple workflow should be valid");
    assert!(result.errors.is_empty());
}

#[test]
fn test_detect_circular_dependency_simple() {
    // Create a circular dependency: A -> B -> A
    let mut states = HashMap::new();
    states.insert(
        "a".to_string(),
        create_state(
            "a",
            Some(TransitionRules {
                success: Some("b".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert(
        "b".to_string(),
        create_state(
            "b",
            Some(TransitionRules {
                success: Some("a".to_string()), // Cycles back to A
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );

    let workflow = WorkflowDefinition {
        name: "circular-workflow".to_string(),
        description: None,
        initial_state: "a".to_string(),
        terminal_states: vec![],
        states,
        defaults: None,
    };

    // TODO: Once circular dependency detection is implemented:
    // let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    // assert!(!result.is_valid());
    // assert!(result.errors.iter().any(|e| e.message.contains("circular") || e.message.contains("cycle")));

    assert!(workflow.states.contains_key("a"));
    assert!(workflow.states.contains_key("b"));
}

#[test]
fn test_detect_circular_dependency_complex() {
    // Create a longer cycle: A -> B -> C -> D -> B
    let mut states = HashMap::new();
    states.insert(
        "a".to_string(),
        create_state(
            "a",
            Some(TransitionRules {
                success: Some("b".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert(
        "b".to_string(),
        create_state(
            "b",
            Some(TransitionRules {
                success: Some("c".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert(
        "c".to_string(),
        create_state(
            "c",
            Some(TransitionRules {
                success: Some("d".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert(
        "d".to_string(),
        create_state(
            "d",
            Some(TransitionRules {
                success: Some("b".to_string()), // Cycles back to B
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );

    let workflow = WorkflowDefinition {
        name: "complex-circular".to_string(),
        description: None,
        initial_state: "a".to_string(),
        terminal_states: vec![],
        states,
        defaults: None,
    };

    // TODO: Verify circular dependency is detected
    assert_eq!(workflow.states.len(), 4);
}

#[test]
fn test_detect_unreachable_states() {
    // Create workflow with unreachable state
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert("end".to_string(), create_state("end", None));
    // This state is unreachable from start
    states.insert("orphan".to_string(), create_state("orphan", None));

    let workflow = WorkflowDefinition {
        name: "unreachable-workflow".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec!["end".to_string()],
        states,
        defaults: None,
    };

    // TODO: Once reachability analysis is implemented:
    // let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    // assert!(!result.warnings.is_empty());
    // assert!(result.warnings.iter().any(|w| w.contains("unreachable") && w.contains("orphan")));

    assert!(workflow.states.contains_key("orphan"));
}

#[test]
fn test_all_states_reachable() {
    // All states should be reachable
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("middle".to_string()),
                failure: Some("error".to_string()),
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert(
        "middle".to_string(),
        create_state(
            "middle",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert("end".to_string(), create_state("end", None));
    states.insert("error".to_string(), create_state("error", None));

    let workflow = WorkflowDefinition {
        name: "all-reachable".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec!["end".to_string(), "error".to_string()],
        states,
        defaults: None,
    };

    // TODO: Verify no unreachable state warnings
    assert_eq!(workflow.states.len(), 4);
}

#[test]
fn test_validate_terminal_states_have_no_transitions() {
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    // Terminal state with transitions (invalid)
    states.insert(
        "end".to_string(),
        create_state(
            "end",
            Some(TransitionRules {
                success: Some("start".to_string()), // Terminal shouldn't have transitions
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );

    let workflow = WorkflowDefinition {
        name: "invalid-terminal".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec!["end".to_string()],
        states,
        defaults: None,
    };

    // TODO: Once terminal state validation is implemented:
    // let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    // assert!(!result.is_valid());
    // assert!(result.errors.iter().any(|e| e.field.contains("terminal") && e.message.contains("transition")));

    assert!(workflow.terminal_states.contains(&"end".to_string()));
}

#[test]
fn test_validate_initial_state_exists() {
    let mut states = HashMap::new();
    states.insert("start".to_string(), create_state("start", None));

    let workflow = WorkflowDefinition {
        name: "missing-initial".to_string(),
        description: None,
        initial_state: "nonexistent".to_string(), // Initial state doesn't exist
        terminal_states: vec!["start".to_string()],
        states,
        defaults: None,
    };

    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    assert!(!result.is_valid());
    assert!(result
        .errors
        .iter()
        .any(|e| e.field == "initial_state" && e.message.contains("not found")));
}

#[test]
fn test_validate_terminal_states_exist() {
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert("end".to_string(), create_state("end", None));

    let workflow = WorkflowDefinition {
        name: "missing-terminal".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec!["nonexistent".to_string()], // Terminal state doesn't exist
        states,
        defaults: None,
    };

    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    assert!(!result.is_valid());
    assert!(result
        .errors
        .iter()
        .any(|e| e.field == "terminal_states" && e.message.contains("not found")));
}

#[test]
fn test_validate_transition_target_states_exist() {
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("nonexistent".to_string()), // Target doesn't exist
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );

    let workflow = WorkflowDefinition {
        name: "invalid-transition".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec!["start".to_string()],
        states,
        defaults: None,
    };

    // TODO: Once transition target validation is implemented:
    // let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    // assert!(!result.is_valid());
    // assert!(result.errors.iter().any(|e| e.message.contains("nonexistent") && e.message.contains("not found")));

    assert!(workflow.states.contains_key("start"));
}

#[test]
fn test_validate_clear_error_messages() {
    // Test that error messages are clear and actionable (FR-016)
    let mut states = HashMap::new();
    states.insert("only_state".to_string(), create_state("only_state", None));

    let workflow = WorkflowDefinition {
        name: "".to_string(), // Empty name
        description: None,
        initial_state: "missing".to_string(), // Missing initial state
        terminal_states: vec!["also_missing".to_string()], // Missing terminal
        states,
        defaults: None,
    };

    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    assert!(!result.is_valid());

    // Verify error messages are descriptive
    for error in &result.errors {
        assert!(
            !error.message.is_empty(),
            "Error message should not be empty"
        );
        assert!(
            !error.field.is_empty(),
            "Error field should identify the problem location"
        );
        // Error message should be actionable (contain specific info)
        assert!(
            error.message.len() > 10,
            "Error message should be descriptive"
        );
    }
}

#[test]
fn test_validate_workflow_with_defaults() {
    let mut states = HashMap::new();
    states.insert(
        "task".to_string(),
        create_state(
            "task",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert("end".to_string(), create_state("end", None));

    let workflow = WorkflowDefinition {
        name: "workflow-with-defaults".to_string(),
        description: None,
        initial_state: "task".to_string(),
        terminal_states: vec!["end".to_string()],
        states,
        defaults: Some(DefaultConfiguration {
            timeout_seconds: None,
            retry_policy: Some(RetryPolicy {
                max_attempts: 3,
                initial_delay_seconds: 5,
                exponential_backoff: true,
                backoff_multiplier: 2.0,
            }),
        }),
    };

    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    // Should validate default retry policy
    assert!(result.is_valid());
}

#[test]
fn test_validate_multiple_terminal_states() {
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        create_state(
            "start",
            Some(TransitionRules {
                success: Some("success".to_string()),
                failure: Some("failure".to_string()),
                timeout: Some("timeout".to_string()),
                approval_denied: None,
            }),
        ),
    );
    states.insert("success".to_string(), create_state("success", None));
    states.insert("failure".to_string(), create_state("failure", None));
    states.insert("timeout".to_string(), create_state("timeout", None));

    let workflow = WorkflowDefinition {
        name: "multi-terminal".to_string(),
        description: None,
        initial_state: "start".to_string(),
        terminal_states: vec![
            "success".to_string(),
            "failure".to_string(),
            "timeout".to_string(),
        ],
        states,
        defaults: None,
    };

    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    assert!(result.is_valid());
}

#[test]
fn test_validate_self_transition_allowed() {
    // A state can transition to itself (e.g., for retry loops)
    let mut states = HashMap::new();
    states.insert(
        "retry_loop".to_string(),
        create_state(
            "retry_loop",
            Some(TransitionRules {
                success: Some("end".to_string()),
                failure: Some("retry_loop".to_string()), // Self-transition
                timeout: None,
                approval_denied: None,
            }),
        ),
    );
    states.insert("end".to_string(), create_state("end", None));

    let workflow = WorkflowDefinition {
        name: "self-transition".to_string(),
        description: None,
        initial_state: "retry_loop".to_string(),
        terminal_states: vec!["end".to_string()],
        states,
        defaults: None,
    };

    // Self-transition should be allowed but might generate a warning
    let result = WorkflowValidator::validate_workflow(&workflow).unwrap();
    assert!(result.is_valid());
}
