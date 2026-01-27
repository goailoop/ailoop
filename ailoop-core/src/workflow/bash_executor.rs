//! Bash command executor for workflow states

use crate::models::workflow::{ExecutionResult, TransitionType, WorkflowState};
use crate::workflow::executor::StateMachineExecutor;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

/// Bash command executor
pub struct BashExecutor {
    // Future: Add output streaming manager
}

impl BashExecutor {
    /// Create new bash executor
    pub fn new() -> Self {
        Self {}
    }

    /// Determine next state based on execution result
    fn determine_next_state(
        &self,
        state: &WorkflowState,
        success: bool,
        timed_out: bool,
    ) -> Option<String> {
        let transitions = state.transitions.as_ref()?;

        if timed_out {
            transitions.timeout.clone()
        } else if success {
            transitions.success.clone()
        } else {
            transitions.failure.clone()
        }
    }

    /// Classify failure as transient or permanent (T061)
    /// Exit codes 1-10 and SIGTERM are transient
    /// Exit codes >10 and validation errors are permanent
    fn is_transient_failure(&self, exit_code: Option<i32>) -> bool {
        match exit_code {
            Some(code) if code >= 1 && code <= 10 => true, // Transient
            Some(code) if code == 143 => true,             // SIGTERM (128 + 15)
            _ => false,                                    // Permanent or unknown
        }
    }

    /// Calculate retry delay with optional exponential backoff (T059)
    fn calculate_retry_delay(
        &self,
        attempt: u32,
        initial_delay_seconds: u32,
        exponential_backoff: bool,
        backoff_multiplier: f64,
    ) -> Duration {
        let delay_seconds = if exponential_backoff {
            // Exponential: initial * (multiplier ^ attempt)
            let delay = initial_delay_seconds as f64 * backoff_multiplier.powi(attempt as i32);
            // Cap at 600 seconds per FR-040
            delay.min(600.0) as u64
        } else {
            // Linear: always use initial delay
            initial_delay_seconds as u64
        };

        Duration::from_secs(delay_seconds)
    }

    /// Execute command with retry logic (T058-T060)
    async fn execute_with_retry(
        &self,
        execution_id: &str,
        state: &WorkflowState,
    ) -> Result<ExecutionResult> {
        let retry_policy = state.retry_policy.as_ref();
        let max_attempts = retry_policy.map(|p| p.max_attempts).unwrap_or(1);

        let mut last_result: Option<ExecutionResult> = None;

        for attempt in 0..max_attempts {
            tracing::debug!(
                "Executing state '{}' (attempt {}/{})",
                state.name,
                attempt + 1,
                max_attempts
            );

            // Execute command
            let result = self.execute_once(execution_id, state).await?;

            // Check if succeeded
            if result.success {
                // Success - return immediately with retry_attempt metadata
                return Ok(ExecutionResult {
                    retry_attempt: if attempt > 0 { Some(attempt + 1) } else { None },
                    ..result
                });
            }

            // Check if this is a permanent failure (skip retry)
            if !self.is_transient_failure(result.exit_code) {
                tracing::warn!(
                    "Permanent failure detected (exit code {:?}) - skipping retry",
                    result.exit_code
                );
                return Ok(ExecutionResult {
                    retry_attempt: Some(attempt + 1),
                    error_message: Some(format!(
                        "Permanent failure (exit code {:?})",
                        result.exit_code
                    )),
                    ..result
                });
            }

            last_result = Some(result);

            // If this was the last attempt, don't delay
            if attempt + 1 >= max_attempts {
                break;
            }

            // Calculate and apply retry delay
            if let Some(policy) = retry_policy {
                let delay = self.calculate_retry_delay(
                    attempt,
                    policy.initial_delay_seconds,
                    policy.exponential_backoff,
                    policy.backoff_multiplier,
                );

                tracing::info!(
                    "Retrying after {} seconds (attempt {}/{})",
                    delay.as_secs(),
                    attempt + 1,
                    max_attempts
                );

                tokio::time::sleep(delay).await;
            }
        }

        // All retries exhausted (T060)
        if let Some(mut final_result) = last_result {
            final_result.retry_attempt = Some(max_attempts);
            final_result.error_message =
                Some(format!("Retry exhausted after {} attempts", max_attempts));
            Ok(final_result)
        } else {
            Err(anyhow::anyhow!("No execution result available"))
        }
    }

    /// Execute command once without retry
    async fn execute_once(
        &self,
        _execution_id: &str,
        state: &WorkflowState,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Get command from state
        let command = state
            .command
            .as_ref()
            .context("State has no command to execute")?;

        // Get timeout (use default if not specified)
        let timeout_seconds = state.timeout_seconds.unwrap_or(300);
        let timeout_duration = Duration::from_secs(timeout_seconds as u64);

        // Spawn bash process
        let mut child = Command::new("bash")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn bash process")?;

        // Get stdout and stderr handles
        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let stderr = child.stderr.take().context("Failed to get stderr")?;

        // Spawn tasks to read output (for now, just consume it)
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!("stdout: {}", line);
            }
        });

        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!("stderr: {}", line);
            }
        });

        // Wait for process with timeout
        let wait_result = timeout(timeout_duration, child.wait()).await;

        // Cleanup output tasks
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        let execution_duration_ms = start_time.elapsed().as_millis() as u64;

        match wait_result {
            Ok(Ok(status)) => {
                // Process completed within timeout
                let success = status.success();
                let exit_code = status.code();

                let next_state = self
                    .determine_next_state(state, success, false)
                    .context("Failed to determine next state")?;

                let transition_type = if success {
                    TransitionType::Success
                } else {
                    TransitionType::Failure
                };

                Ok(ExecutionResult {
                    success,
                    exit_code,
                    execution_duration_ms,
                    next_state,
                    transition_type,
                    retry_attempt: None,
                    error_message: if success {
                        None
                    } else {
                        Some(format!("Command failed with exit code {:?}", exit_code))
                    },
                })
            }
            Ok(Err(e)) => {
                // Error waiting for process
                Err(e).context("Failed to wait for process")
            }
            Err(_) => {
                // Timeout - kill the process
                let _ = child.kill().await;

                let next_state = self
                    .determine_next_state(state, false, true)
                    .context("Failed to determine next state after timeout")?;

                Ok(ExecutionResult {
                    success: false,
                    exit_code: None,
                    execution_duration_ms,
                    next_state,
                    transition_type: TransitionType::Timeout,
                    retry_attempt: None,
                    error_message: Some(format!(
                        "Command timed out after {} seconds",
                        timeout_seconds
                    )),
                })
            }
        }
    }
}

impl Default for BashExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateMachineExecutor for BashExecutor {
    async fn execute(&self, execution_id: &str, state: &WorkflowState) -> Result<ExecutionResult> {
        // Use execute_with_retry which handles both retry logic and single execution
        self.execute_with_retry(execution_id, state).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::workflow::{TransitionRules, WorkflowState};

    #[tokio::test]
    async fn test_bash_executor_success() {
        let executor = BashExecutor::new();

        let state = WorkflowState {
            name: "test".to_string(),
            description: "Test state".to_string(),
            command: Some("echo 'hello' && exit 0".to_string()),
            timeout_seconds: Some(10),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: Some(TransitionRules {
                success: Some("next".to_string()),
                failure: Some("failed".to_string()),
                timeout: None,
                approval_denied: None,
            }),
            timeout_behavior: crate::models::workflow::TimeoutBehavior::DenyAndFail,
        };

        let result = executor.execute("test-exec", &state).await.unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.next_state, "next");
        assert_eq!(result.transition_type, TransitionType::Success);
    }

    #[tokio::test]
    async fn test_bash_executor_failure() {
        let executor = BashExecutor::new();

        let state = WorkflowState {
            name: "test".to_string(),
            description: "Test state".to_string(),
            command: Some("exit 1".to_string()),
            timeout_seconds: Some(10),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: Some(TransitionRules {
                success: Some("next".to_string()),
                failure: Some("failed".to_string()),
                timeout: None,
                approval_denied: None,
            }),
            timeout_behavior: crate::models::workflow::TimeoutBehavior::DenyAndFail,
        };

        let result = executor.execute("test-exec", &state).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(result.next_state, "failed");
        assert_eq!(result.transition_type, TransitionType::Failure);
    }

    #[tokio::test]
    async fn test_bash_executor_timeout() {
        let executor = BashExecutor::new();

        let state = WorkflowState {
            name: "test".to_string(),
            description: "Test state".to_string(),
            command: Some("sleep 10".to_string()),
            timeout_seconds: Some(1),
            requires_approval: false,
            approval_timeout: None,
            approval_description: None,
            retry_policy: None,
            transitions: Some(TransitionRules {
                success: Some("next".to_string()),
                failure: Some("failed".to_string()),
                timeout: Some("timeout".to_string()),
                approval_denied: None,
            }),
            timeout_behavior: crate::models::workflow::TimeoutBehavior::DenyAndFail,
        };

        let result = executor.execute("test-exec", &state).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.next_state, "timeout");
        assert_eq!(result.transition_type, TransitionType::Timeout);
    }
}
