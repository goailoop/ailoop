//! Workflow validation logic

use crate::models::workflow::{RetryPolicy, WorkflowDefinition};
use anyhow::Result;

/// Validation error type
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Validation result
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, field: String, message: String) {
        self.errors.push(ValidationError { field, message });
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Workflow validator
pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Validate retry policy (T064)
    /// - max_attempts: 1-10
    /// - initial_delay: 1-300s
    /// - backoff_multiplier >= 1.0
    /// - max delay cap: 600s (enforced in calculation, not validated here)
    pub fn validate_retry_policy(policy: &RetryPolicy) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Validate max_attempts (1-10)
        if policy.max_attempts < 1 {
            result.add_error(
                "max_attempts".to_string(),
                "max_attempts must be at least 1".to_string(),
            );
        }
        if policy.max_attempts > 10 {
            result.add_error(
                "max_attempts".to_string(),
                "max_attempts cannot exceed 10".to_string(),
            );
        }

        // Validate initial_delay_seconds (1-300)
        if policy.initial_delay_seconds < 1 {
            result.add_error(
                "initial_delay_seconds".to_string(),
                "initial_delay_seconds must be at least 1".to_string(),
            );
        }
        if policy.initial_delay_seconds > 300 {
            result.add_error(
                "initial_delay_seconds".to_string(),
                "initial_delay_seconds cannot exceed 300".to_string(),
            );
        }

        // Validate backoff_multiplier (>= 1.0)
        if policy.backoff_multiplier < 1.0 {
            result.add_error(
                "backoff_multiplier".to_string(),
                "backoff_multiplier must be at least 1.0".to_string(),
            );
        }

        // Warn if exponential backoff might exceed max delay quickly
        if policy.exponential_backoff && policy.backoff_multiplier > 3.0 {
            result.add_warning(format!(
                "High backoff_multiplier ({}) with exponential backoff may reach max delay (600s) quickly",
                policy.backoff_multiplier
            ));
        }

        Ok(result)
    }

    /// Validate workflow definition
    pub fn validate_workflow(workflow: &WorkflowDefinition) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Validate workflow name
        if workflow.name.is_empty() {
            result.add_error(
                "name".to_string(),
                "Workflow name cannot be empty".to_string(),
            );
        }

        // Validate initial state exists
        if !workflow.states.contains_key(&workflow.initial_state) {
            result.add_error(
                "initial_state".to_string(),
                format!(
                    "Initial state '{}' not found in states",
                    workflow.initial_state
                ),
            );
        }

        // Validate terminal states exist
        for terminal_state in &workflow.terminal_states {
            if !workflow.states.contains_key(terminal_state) {
                result.add_error(
                    "terminal_states".to_string(),
                    format!("Terminal state '{}' not found in states", terminal_state),
                );
            }
        }

        // Validate state retry policies
        for (state_name, state) in &workflow.states {
            if let Some(retry_policy) = &state.retry_policy {
                let policy_validation = Self::validate_retry_policy(retry_policy)?;
                for error in policy_validation.errors {
                    result.add_error(
                        format!("states.{}.retry_policy.{}", state_name, error.field),
                        error.message,
                    );
                }
                for warning in policy_validation.warnings {
                    result.add_warning(format!("State '{}': {}", state_name, warning));
                }
            }
        }

        // Validate default retry policy if present
        if let Some(defaults) = &workflow.defaults {
            if let Some(default_retry_policy) = &defaults.retry_policy {
                let policy_validation = Self::validate_retry_policy(default_retry_policy)?;
                for error in policy_validation.errors {
                    result.add_error(
                        format!("defaults.retry_policy.{}", error.field),
                        error.message,
                    );
                }
                for warning in policy_validation.warnings {
                    result.add_warning(format!("Default retry policy: {}", warning));
                }
            }
        }

        // T092: Check for circular dependencies
        if let Some(cycle) = Self::detect_circular_dependencies(workflow) {
            result.add_error(
                "states".to_string(),
                format!(
                    "Circular dependency detected: {} → {}",
                    cycle.join(" → "),
                    cycle[0]
                ),
            );
        }

        // T093: Check for unreachable states
        let unreachable = Self::find_unreachable_states(workflow);
        for state_name in unreachable {
            result.add_warning(format!(
                "State '{}' is unreachable from initial state '{}'",
                state_name, workflow.initial_state
            ));
        }

        // T094: Validate terminal states have no outgoing transitions
        for terminal_state in &workflow.terminal_states {
            if let Some(state) = workflow.states.get(terminal_state) {
                if state.transitions.is_some() {
                    result.add_error(
                        format!("states.{}.transitions", terminal_state),
                        format!(
                            "Terminal state '{}' should not have outgoing transitions",
                            terminal_state
                        ),
                    );
                }
            }
        }

        // T103: Warn if non-terminal states are missing success/failure transitions
        for (state_name, state) in &workflow.states {
            if !workflow.terminal_states.contains(state_name) && state.command.is_some() {
                if let Some(transitions) = &state.transitions {
                    if transitions.success.is_none() {
                        result.add_warning(format!(
                            "State '{}' has a command but no success transition defined",
                            state_name
                        ));
                    }
                    if transitions.failure.is_none() {
                        result.add_warning(format!(
                            "State '{}' has a command but no failure transition defined",
                            state_name
                        ));
                    }
                } else {
                    result.add_warning(format!(
                        "Non-terminal state '{}' has a command but no transitions defined",
                        state_name
                    ));
                }
            }
        }

        // Validate transition targets exist
        for (state_name, state) in &workflow.states {
            if let Some(transitions) = &state.transitions {
                if let Some(ref target) = transitions.success {
                    if !workflow.states.contains_key(target) {
                        result.add_error(
                            format!("states.{}.transitions.success", state_name),
                            format!("Transition target state '{}' not found", target),
                        );
                    }
                }
                if let Some(ref target) = transitions.failure {
                    if !workflow.states.contains_key(target) {
                        result.add_error(
                            format!("states.{}.transitions.failure", state_name),
                            format!("Transition target state '{}' not found", target),
                        );
                    }
                }
                if let Some(ref target) = transitions.timeout {
                    if !workflow.states.contains_key(target) {
                        result.add_error(
                            format!("states.{}.transitions.timeout", state_name),
                            format!("Transition target state '{}' not found", target),
                        );
                    }
                }
                if let Some(ref target) = transitions.approval_denied {
                    if !workflow.states.contains_key(target) {
                        result.add_error(
                            format!("states.{}.transitions.approval_denied", state_name),
                            format!("Transition target state '{}' not found", target),
                        );
                    }
                }
            }
        }

        Ok(result)
    }

    /// Detect circular dependencies using DFS cycle detection (T092)
    fn detect_circular_dependencies(workflow: &WorkflowDefinition) -> Option<Vec<String>> {
        use std::collections::{HashMap, HashSet};

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut parent = HashMap::new();

        fn dfs(
            node: &str,
            workflow: &WorkflowDefinition,
            visited: &mut HashSet<String>,
            rec_stack: &mut HashSet<String>,
            parent: &mut HashMap<String, String>,
        ) -> Option<Vec<String>> {
            visited.insert(node.to_string());
            rec_stack.insert(node.to_string());

            // Get all transition targets
            if let Some(state) = workflow.states.get(node) {
                if let Some(transitions) = &state.transitions {
                    let targets: Vec<&String> = vec![
                        transitions.success.as_ref(),
                        transitions.failure.as_ref(),
                        transitions.timeout.as_ref(),
                        transitions.approval_denied.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    for target in targets {
                        if !visited.contains(target) {
                            parent.insert(target.clone(), node.to_string());
                            if let Some(cycle) = dfs(target, workflow, visited, rec_stack, parent) {
                                return Some(cycle);
                            }
                        } else if rec_stack.contains(target) {
                            // Cycle detected
                            if target == node {
                                // Self-loop is allowed (for retry loops)
                                continue;
                            }
                            // Reconstruct path for non-self cycles
                            let mut cycle = vec![target.clone()];
                            let mut current = node;
                            while current != target {
                                cycle.push(current.to_string());
                                current = parent.get(current)?;
                            }
                            cycle.reverse();
                            return Some(cycle);
                        }
                    }
                }
            }

            rec_stack.remove(node);
            None
        }

        // Start DFS from initial state
        dfs(
            &workflow.initial_state,
            workflow,
            &mut visited,
            &mut rec_stack,
            &mut parent,
        )
    }

    /// Find unreachable states using BFS (T093)
    fn find_unreachable_states(workflow: &WorkflowDefinition) -> Vec<String> {
        use std::collections::{HashSet, VecDeque};

        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        // Start from initial state
        queue.push_back(workflow.initial_state.clone());
        reachable.insert(workflow.initial_state.clone());

        // BFS to find all reachable states
        while let Some(state_name) = queue.pop_front() {
            if let Some(state) = workflow.states.get(&state_name) {
                if let Some(transitions) = &state.transitions {
                    let targets: Vec<&String> = vec![
                        transitions.success.as_ref(),
                        transitions.failure.as_ref(),
                        transitions.timeout.as_ref(),
                        transitions.approval_denied.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    for target in targets {
                        if reachable.insert(target.clone()) {
                            queue.push_back(target.clone());
                        }
                    }
                }
            }
        }

        // Find states that are not reachable
        workflow
            .states
            .keys()
            .filter(|state_name| !reachable.contains(*state_name))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_retry_policy_valid() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_seconds: 5,
            exponential_backoff: true,
            backoff_multiplier: 2.0,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_retry_policy_max_attempts_too_low() {
        let policy = RetryPolicy {
            max_attempts: 0,
            initial_delay_seconds: 5,
            exponential_backoff: false,
            backoff_multiplier: 1.0,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("at least 1"));
    }

    #[test]
    fn test_validate_retry_policy_max_attempts_too_high() {
        let policy = RetryPolicy {
            max_attempts: 15,
            initial_delay_seconds: 5,
            exponential_backoff: false,
            backoff_multiplier: 1.0,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("cannot exceed 10"));
    }

    #[test]
    fn test_validate_retry_policy_delay_too_low() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_seconds: 0,
            exponential_backoff: false,
            backoff_multiplier: 1.0,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("at least 1"));
    }

    #[test]
    fn test_validate_retry_policy_delay_too_high() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_seconds: 500,
            exponential_backoff: false,
            backoff_multiplier: 1.0,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("cannot exceed 300"));
    }

    #[test]
    fn test_validate_retry_policy_multiplier_too_low() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_seconds: 5,
            exponential_backoff: true,
            backoff_multiplier: 0.5,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("at least 1.0"));
    }

    #[test]
    fn test_validate_retry_policy_high_multiplier_warning() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_seconds: 5,
            exponential_backoff: true,
            backoff_multiplier: 5.0,
        };

        let result = WorkflowValidator::validate_retry_policy(&policy).unwrap();
        assert!(result.is_valid()); // Valid but has warnings
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("High backoff_multiplier"));
    }
}
