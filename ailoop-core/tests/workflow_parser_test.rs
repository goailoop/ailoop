//! Unit tests for YAML workflow parser

use ailoop_core::models::workflow::{
    RetryPolicy, TimeoutBehavior, TransitionRules, WorkflowDefinition, WorkflowState,
};
use std::collections::HashMap;

#[test]
fn test_parse_simple_workflow() {
    // Simple workflow YAML
    let yaml = r#"
name: test-workflow
description: A simple test workflow
initial_state: start
terminal_states:
  - completed
  - failed

states:
  start:
    name: start
    description: Starting state
    command: "echo 'Starting'"
    timeout_seconds: 30
    transitions:
      success: completed
      failure: failed

  completed:
    name: completed
    description: Success state

  failed:
    name: failed
    description: Failure state
"#;

    let result: Result<WorkflowDefinition, serde_yaml::Error> = serde_yaml::from_str(yaml);
    assert!(result.is_ok());

    let workflow = result.unwrap();
    assert_eq!(workflow.name, "test-workflow");
    assert_eq!(workflow.initial_state, "start");
    assert_eq!(workflow.terminal_states.len(), 2);
    assert_eq!(workflow.states.len(), 3);
    assert!(workflow.states.contains_key("start"));
}

#[test]
fn test_parse_workflow_with_retry_policy() {
    let yaml = r#"
name: retry-workflow
initial_state: task
terminal_states:
  - done

defaults:
  retry_policy:
    max_attempts: 3
    initial_delay_seconds: 5
    exponential_backoff: true
    backoff_multiplier: 2.0

states:
  task:
    name: task
    description: Task with retry
    command: "echo 'task'"
    transitions:
      success: done

  done:
    name: done
    description: Done
"#;

    let result: Result<WorkflowDefinition, serde_yaml::Error> = serde_yaml::from_str(yaml);
    assert!(result.is_ok());

    let workflow = result.unwrap();
    assert!(workflow.defaults.is_some());
    let defaults = workflow.defaults.unwrap();
    assert!(defaults.retry_policy.is_some());

    let retry_policy = defaults.retry_policy.unwrap();
    assert_eq!(retry_policy.max_attempts, 3);
    assert_eq!(retry_policy.initial_delay_seconds, 5);
    assert!(retry_policy.exponential_backoff);
    assert_eq!(retry_policy.backoff_multiplier, 2.0);
}

#[test]
fn test_parse_workflow_with_state_retry_policy() {
    let yaml = r#"
name: state-retry-workflow
initial_state: flaky_task
terminal_states:
  - done

states:
  flaky_task:
    name: flaky_task
    description: Flaky task
    command: "echo 'flaky'"
    retry_policy:
      max_attempts: 5
      initial_delay_seconds: 2
      exponential_backoff: false
      backoff_multiplier: 1.0
    transitions:
      success: done

  done:
    name: done
    description: Done
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
    let state = workflow.states.get("flaky_task").unwrap();
    assert!(state.retry_policy.is_some());

    let retry_policy = state.retry_policy.as_ref().unwrap();
    assert_eq!(retry_policy.max_attempts, 5);
    assert!(!retry_policy.exponential_backoff);
}

#[test]
fn test_parse_workflow_with_approval() {
    let yaml = r#"
name: approval-workflow
initial_state: validate
terminal_states:
  - deployed
  - cancelled

states:
  validate:
    name: validate
    description: Validation
    command: "echo 'validating'"
    transitions:
      success: approval_gate

  approval_gate:
    name: approval_gate
    description: Approval required
    requires_approval: true
    approval_timeout: 300
    approval_description: "Deploy to production?"
    transitions:
      success: deployed
      approval_denied: cancelled

  deployed:
    name: deployed
    description: Deployed

  cancelled:
    name: cancelled
    description: Cancelled
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
    let approval_state = workflow.states.get("approval_gate").unwrap();
    assert!(approval_state.requires_approval);
    assert_eq!(approval_state.approval_timeout, Some(300));
    assert_eq!(
        approval_state.approval_description,
        Some("Deploy to production?".to_string())
    );
}

#[test]
fn test_parse_invalid_yaml() {
    let invalid_yaml = r#"
name: broken-workflow
initial_state: start
states:
  - this is not valid YAML structure
"#;

    let result: Result<WorkflowDefinition, serde_yaml::Error> = serde_yaml::from_str(invalid_yaml);
    assert!(result.is_err());
}

#[test]
fn test_parse_missing_required_fields() {
    // Missing 'name' field
    let yaml = r#"
initial_state: start
terminal_states:
  - done
states:
  start:
    name: start
    description: Start
  done:
    name: done
    description: Done
"#;

    let result: Result<WorkflowDefinition, serde_yaml::Error> = serde_yaml::from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_parse_workflow_with_timeout_behavior() {
    let yaml = r#"
name: timeout-workflow
initial_state: task
terminal_states:
  - done
  - timeout

states:
  task:
    name: task
    description: Task with timeout
    command: "sleep 10"
    timeout_seconds: 5
    timeout_behavior: deny_and_fail
    transitions:
      success: done
      timeout: timeout

  done:
    name: done
    description: Done

  timeout:
    name: timeout
    description: Timed out
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
    let task_state = workflow.states.get("task").unwrap();
    assert_eq!(task_state.timeout_seconds, Some(5));
    assert_eq!(task_state.timeout_behavior, TimeoutBehavior::DenyAndFail);
}

#[test]
fn test_parse_workflow_with_all_transition_types() {
    let yaml = r#"
name: full-transitions
initial_state: main_task
terminal_states:
  - success_end
  - failure_end
  - timeout_end
  - denied_end

states:
  main_task:
    name: main_task
    description: Main task
    command: "echo 'main'"
    requires_approval: true
    timeout_seconds: 60
    transitions:
      success: success_end
      failure: failure_end
      timeout: timeout_end
      approval_denied: denied_end

  success_end:
    name: success_end
    description: Success

  failure_end:
    name: failure_end
    description: Failed

  timeout_end:
    name: timeout_end
    description: Timed out

  denied_end:
    name: denied_end
    description: Denied
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
    let main_state = workflow.states.get("main_task").unwrap();
    let transitions = main_state.transitions.as_ref().unwrap();

    assert_eq!(transitions.success, Some("success_end".to_string()));
    assert_eq!(transitions.failure, Some("failure_end".to_string()));
    assert_eq!(transitions.timeout, Some("timeout_end".to_string()));
    assert_eq!(transitions.approval_denied, Some("denied_end".to_string()));
}

#[test]
fn test_parse_workflow_deserialization_roundtrip() {
    // Create a workflow programmatically
    let mut states = HashMap::new();
    states.insert(
        "start".to_string(),
        WorkflowState {
            name: "start".to_string(),
            description: "Start".to_string(),
            command: Some("echo 'start'".to_string()),
            timeout_seconds: Some(30),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: Some(RetryPolicy {
                max_attempts: 3,
                initial_delay_seconds: 5,
                exponential_backoff: true,
                backoff_multiplier: 2.0,
            }),
            transitions: Some(TransitionRules {
                success: Some("end".to_string()),
                failure: None,
                timeout: None,
                approval_denied: None,
            }),
            timeout_behavior: TimeoutBehavior::DenyAndFail,
        },
    );
    states.insert(
        "end".to_string(),
        WorkflowState {
            name: "end".to_string(),
            description: "End".to_string(),
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

    let original = WorkflowDefinition {
        name: "roundtrip-test".to_string(),
        description: Some("Test roundtrip".to_string()),
        initial_state: "start".to_string(),
        terminal_states: vec!["end".to_string()],
        states,
        defaults: None,
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&original).unwrap();

    // Deserialize back
    let deserialized: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(deserialized.name, original.name);
    assert_eq!(deserialized.initial_state, original.initial_state);
    assert_eq!(deserialized.states.len(), original.states.len());
}

#[test]
fn test_parse_empty_workflow() {
    let yaml = r#"
name: empty
initial_state: only_state
terminal_states:
  - only_state

states:
  only_state:
    name: only_state
    description: Only state
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(workflow.states.len(), 1);
    assert_eq!(workflow.initial_state, "only_state");
    assert_eq!(workflow.terminal_states, vec!["only_state"]);
}
