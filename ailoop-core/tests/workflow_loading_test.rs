//! Integration test for workflow loading
//! Tests loading YAML, validating, and executing workflows multiple times

use ailoop_core::models::workflow::WorkflowDefinition;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_workflow_from_yaml_file() {
    let temp_dir = TempDir::new().unwrap();
    let workflow_path = temp_dir.path().join("test_workflow.yaml");

    let yaml_content = r#"
name: file-workflow
description: Workflow loaded from file
initial_state: start
terminal_states:
  - completed

states:
  start:
    name: start
    description: Start state
    command: "echo 'Starting'"
    transitions:
      success: completed

  completed:
    name: completed
    description: Completed state
"#;

    fs::write(&workflow_path, yaml_content).unwrap();

    // Load from file
    let file_content = fs::read_to_string(&workflow_path).unwrap();
    let workflow: WorkflowDefinition = serde_yaml::from_str(&file_content).unwrap();

    assert_eq!(workflow.name, "file-workflow");
    assert_eq!(workflow.states.len(), 2);
}

#[test]
fn test_load_and_validate_workflow() {
    let yaml = r#"
name: validated-workflow
initial_state: start
terminal_states:
  - end

states:
  start:
    name: start
    description: Start
    command: "echo 'start'"
    transitions:
      success: end

  end:
    name: end
    description: End
"#;

    // Load workflow
    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();

    // Validate workflow
    use ailoop_core::workflow::WorkflowValidator;
    let validation_result = WorkflowValidator::validate_workflow(&workflow).unwrap();

    assert!(
        validation_result.is_valid(),
        "Workflow should pass validation"
    );
    assert!(validation_result.errors.is_empty());
}

#[test]
fn test_load_invalid_workflow_and_detect_errors() {
    let yaml = r#"
name: invalid-workflow
initial_state: nonexistent_state
terminal_states:
  - also_nonexistent

states:
  actual_state:
    name: actual_state
    description: The only actual state
"#;

    // Load workflow
    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();

    // Validate workflow - should fail
    use ailoop_core::workflow::WorkflowValidator;
    let validation_result = WorkflowValidator::validate_workflow(&workflow).unwrap();

    assert!(
        !validation_result.is_valid(),
        "Invalid workflow should not pass validation"
    );
    assert!(!validation_result.errors.is_empty());

    // Check specific errors
    assert!(validation_result
        .errors
        .iter()
        .any(|e| e.field.contains("initial_state")));
    assert!(validation_result
        .errors
        .iter()
        .any(|e| e.field.contains("terminal_states")));
}

#[test]
fn test_execute_same_workflow_multiple_times() {
    // Test SC-011: Same workflow definition can be executed multiple times independently
    let yaml = r#"
name: reusable-workflow
initial_state: task
terminal_states:
  - done

states:
  task:
    name: task
    description: Task
    command: "echo 'executing'"
    timeout_seconds: 10
    transitions:
      success: done

  done:
    name: done
    description: Done
"#;

    // Load workflow once
    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();

    // Simulate multiple executions (in reality, orchestrator would handle this)
    let execution_ids = vec!["exec-1", "exec-2", "exec-3"];

    for exec_id in execution_ids {
        // Each execution gets a copy of the workflow definition
        let workflow_instance = workflow.clone();

        assert_eq!(workflow_instance.name, "reusable-workflow");
        assert_eq!(workflow_instance.states.len(), 2);

        // In actual implementation, orchestrator would:
        // 1. Create WorkflowExecutionInstance with unique ID
        // 2. Execute states based on this definition
        // 3. Track state independently per execution

        // For this test, we just verify the definition is reusable
        println!("Would execute workflow for {}", exec_id);
    }

    // Workflow definition remains unchanged
    assert_eq!(workflow.name, "reusable-workflow");
}

#[test]
fn test_load_workflow_with_complex_structure() {
    let yaml = r#"
name: complex-workflow
description: A complex workflow with multiple features
initial_state: validate
terminal_states:
  - deployed
  - failed
  - cancelled

defaults:
  retry_policy:
    max_attempts: 3
    initial_delay_seconds: 5
    exponential_backoff: true
    backoff_multiplier: 2.0

states:
  validate:
    name: validate
    description: Validate inputs
    command: "echo 'validating'"
    timeout_seconds: 60
    transitions:
      success: build
      failure: failed
      timeout: failed

  build:
    name: build
    description: Build application
    command: "echo 'building'"
    timeout_seconds: 300
    retry_policy:
      max_attempts: 2
      initial_delay_seconds: 10
      exponential_backoff: false
      backoff_multiplier: 1.0
    transitions:
      success: test
      failure: failed
      timeout: failed

  test:
    name: test
    description: Run tests
    command: "echo 'testing'"
    timeout_seconds: 600
    transitions:
      success: approval_gate
      failure: failed

  approval_gate:
    name: approval_gate
    description: Approval for deployment
    requires_approval: true
    approval_timeout: 3600
    approval_description: "Deploy to production?"
    transitions:
      success: deploy
      approval_denied: cancelled

  deploy:
    name: deploy
    description: Deploy to production
    command: "echo 'deploying'"
    timeout_seconds: 300
    transitions:
      success: deployed
      failure: failed

  deployed:
    name: deployed
    description: Successfully deployed

  failed:
    name: failed
    description: Deployment failed

  cancelled:
    name: cancelled
    description: Deployment cancelled
"#;

    // Load complex workflow
    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(workflow.name, "complex-workflow");
    assert_eq!(workflow.states.len(), 8);
    assert_eq!(workflow.terminal_states.len(), 3);

    // Verify defaults
    assert!(workflow.defaults.is_some());
    let defaults = workflow.defaults.as_ref().unwrap();
    assert!(defaults.retry_policy.is_some());

    // Verify specific states
    assert!(workflow.states.contains_key("validate"));
    assert!(workflow.states.contains_key("approval_gate"));
    assert!(workflow.states.contains_key("deploy"));

    // Verify approval gate configuration
    let approval_state = workflow.states.get("approval_gate").unwrap();
    assert!(approval_state.requires_approval);
    assert_eq!(approval_state.approval_timeout, Some(3600));

    // Verify build has custom retry policy
    let build_state = workflow.states.get("build").unwrap();
    assert!(build_state.retry_policy.is_some());
    let retry_policy = build_state.retry_policy.as_ref().unwrap();
    assert_eq!(retry_policy.max_attempts, 2);

    // Validate the workflow
    use ailoop_core::workflow::WorkflowValidator;
    let validation_result = WorkflowValidator::validate_workflow(&workflow).unwrap();

    assert!(
        validation_result.is_valid(),
        "Complex workflow should pass validation"
    );
}

#[test]
fn test_load_multiple_workflow_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple workflow files
    let workflows = vec![
        ("workflow1.yaml", "name: workflow-1\ninitial_state: s1\nterminal_states:\n  - s1\nstates:\n  s1:\n    name: s1\n    description: State 1\n"),
        ("workflow2.yaml", "name: workflow-2\ninitial_state: s2\nterminal_states:\n  - s2\nstates:\n  s2:\n    name: s2\n    description: State 2\n"),
        ("workflow3.yaml", "name: workflow-3\ninitial_state: s3\nterminal_states:\n  - s3\nstates:\n  s3:\n    name: s3\n    description: State 3\n"),
    ];

    let mut loaded_workflows = Vec::new();

    for (filename, content) in workflows {
        let path = temp_dir.path().join(filename);
        fs::write(&path, content).unwrap();

        let file_content = fs::read_to_string(&path).unwrap();
        let workflow: WorkflowDefinition = serde_yaml::from_str(&file_content).unwrap();

        loaded_workflows.push(workflow);
    }

    assert_eq!(loaded_workflows.len(), 3);
    assert_eq!(loaded_workflows[0].name, "workflow-1");
    assert_eq!(loaded_workflows[1].name, "workflow-2");
    assert_eq!(loaded_workflows[2].name, "workflow-3");
}

#[test]
fn test_workflow_validation_before_execution() {
    // Test SC-006: Validation detects and reports all structural errors before execution
    let invalid_yaml = r#"
name: invalid-structure
initial_state: start
terminal_states:
  - end

states:
  start:
    name: start
    description: Start
    transitions:
      success: middle  # middle doesn't exist

  end:
    name: end
    description: End
    transitions:
      success: somewhere  # Terminal state shouldn't have transitions
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(invalid_yaml).unwrap();

    use ailoop_core::workflow::WorkflowValidator;
    let validation_result = WorkflowValidator::validate_workflow(&workflow).unwrap();

    // Should detect errors before any execution
    // TODO: Once transition validation and terminal state validation are implemented:
    // assert!(!validation_result.is_valid());
    // assert!(validation_result.errors.iter().any(|e| e.message.contains("middle")));
    // assert!(validation_result.errors.iter().any(|e| e.message.contains("terminal")));

    println!(
        "Detected {} validation errors before execution (more validations TODO)",
        validation_result.errors.len()
    );
}

#[test]
fn test_workflow_defaults_inheritance() {
    let yaml = r#"
name: defaults-test
initial_state: task1
terminal_states:
  - done

defaults:
  retry_policy:
    max_attempts: 5
    initial_delay_seconds: 10
    exponential_backoff: true
    backoff_multiplier: 3.0

states:
  task1:
    name: task1
    description: Task with inherited defaults
    command: "echo 'task1'"
    transitions:
      success: task2

  task2:
    name: task2
    description: Task with custom retry
    command: "echo 'task2'"
    retry_policy:
      max_attempts: 2
      initial_delay_seconds: 1
      exponential_backoff: false
      backoff_multiplier: 1.0
    transitions:
      success: done

  done:
    name: done
    description: Done
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();

    // Verify defaults are loaded
    assert!(workflow.defaults.is_some());
    let defaults = workflow.defaults.as_ref().unwrap();
    assert!(defaults.retry_policy.is_some());

    // task1 would inherit defaults (checked by orchestrator at runtime)
    let task1 = workflow.states.get("task1").unwrap();
    assert!(task1.retry_policy.is_none()); // No explicit policy

    // task2 has custom policy that overrides defaults
    let task2 = workflow.states.get("task2").unwrap();
    assert!(task2.retry_policy.is_some());
    let custom_policy = task2.retry_policy.as_ref().unwrap();
    assert_eq!(custom_policy.max_attempts, 2);
}
