use std::io::Write;
use std::process::{Command, Stdio};

pub fn run_ailoop(args: &[&str]) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["run", "--bin", "ailoop", "--"])
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run ailoop: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run ailoop with specific stdin bytes. Returns (exit_success, stdout, stderr).
pub fn run_ailoop_with_stdin(args: &[&str], stdin_bytes: &[u8]) -> (bool, String, String) {
    let mut child = Command::new("cargo")
        .args(["run", "-q", "--bin", "ailoop", "--"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn ailoop");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_bytes);
    }

    let output = child.wait_with_output().expect("Failed to wait for ailoop");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

pub fn get_help_text() -> Result<String, String> {
    run_ailoop(&["--help", ""])
}

pub fn get_version_text() -> Result<String, String> {
    run_ailoop(&["--version", ""])
}

#[cfg(test)]
mod authorize_timeout_tests {
    use super::*;

    /// Authorize with --default yes and empty input (EOF) should be GRANTED (exit 0).
    /// Empty input triggers parse_authorization_response("", true) -> Approved,
    /// validating that the default-yes path produces an authorized outcome.
    #[test]
    fn test_authorize_default_yes_empty_input_granted() {
        let (success, stdout, _stderr) = run_ailoop_with_stdin(
            &[
                "authorize",
                "test-action",
                "--timeout",
                "0",
                "--default",
                "yes",
            ],
            b"\n",
        );
        assert!(
            success,
            "authorize --default yes with empty input should exit 0 (GRANTED), stdout: {}",
            stdout
        );
        assert!(
            stdout.contains("GRANTED"),
            "stdout should indicate GRANTED, got: {}",
            stdout
        );
    }

    /// Authorize with --default no and empty input (EOF) should be DENIED (exit non-zero).
    #[test]
    fn test_authorize_default_no_empty_input_denied() {
        let (success, stdout, _stderr) = run_ailoop_with_stdin(
            &[
                "authorize",
                "test-action",
                "--timeout",
                "0",
                "--default",
                "no",
            ],
            b"\n",
        );
        assert!(
            !success,
            "authorize --default no with empty input should exit non-zero (DENIED), stdout: {}",
            stdout
        );
        assert!(
            stdout.contains("DENIED"),
            "stdout should indicate DENIED, got: {}",
            stdout
        );
    }

    /// Explicit "yes" input overrides --default no -> GRANTED.
    #[test]
    fn test_authorize_explicit_yes_overrides_default_no() {
        let (success, stdout, _stderr) = run_ailoop_with_stdin(
            &[
                "authorize",
                "test-action",
                "--timeout",
                "0",
                "--default",
                "no",
            ],
            b"yes\n",
        );
        assert!(
            success,
            "authorize with explicit 'yes' should exit 0 regardless of --default, stdout: {}",
            stdout
        );
        assert!(
            stdout.contains("GRANTED"),
            "stdout should indicate GRANTED, got: {}",
            stdout
        );
    }

    /// Explicit "no" input overrides --default yes -> DENIED.
    #[test]
    fn test_authorize_explicit_no_overrides_default_yes() {
        let (success, stdout, _stderr) = run_ailoop_with_stdin(
            &[
                "authorize",
                "test-action",
                "--timeout",
                "0",
                "--default",
                "yes",
            ],
            b"no\n",
        );
        assert!(
            !success,
            "authorize with explicit 'no' should exit non-zero regardless of --default, stdout: {}",
            stdout
        );
        assert!(
            stdout.contains("DENIED"),
            "stdout should indicate DENIED, got: {}",
            stdout
        );
    }

    /// Tests the actual InputResult::Timeout path: --default yes + timeout fires -> GRANTED.
    /// Stdin pipe is kept open so the process cannot receive EOF; only the countdown
    /// timer fires, exercising the InputResult::Timeout branch end-to-end.
    #[test]
    fn test_authorize_default_yes_actual_timeout_path_granted() {
        use std::process::Stdio;

        let mut child = Command::new("cargo")
            .args(["run", "-q", "--bin", "ailoop", "--"])
            .args([
                "authorize",
                "test-action",
                "--timeout",
                "1",
                "--default",
                "yes",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn ailoop");

        // Take stdin but do NOT write or close it so the process hits InputResult::Timeout
        let _stdin = child.stdin.take();

        let output = child.wait_with_output().expect("Failed to wait for ailoop");
        drop(_stdin);

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(
            output.status.success(),
            "authorize --default yes should exit 0 on timeout (InputResult::Timeout path), stdout: {}, stderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("GRANTED"),
            "stdout should indicate GRANTED on timeout with --default yes, got: {}",
            stdout
        );
    }

    /// Tests the actual InputResult::Timeout path: --default no + timeout fires -> DENIED.
    /// Stdin pipe is kept open so the process cannot receive EOF; only the countdown
    /// timer fires, exercising the InputResult::Timeout branch end-to-end.
    #[test]
    fn test_authorize_default_no_actual_timeout_path_denied() {
        use std::process::Stdio;

        let mut child = Command::new("cargo")
            .args(["run", "-q", "--bin", "ailoop", "--"])
            .args([
                "authorize",
                "test-action",
                "--timeout",
                "1",
                "--default",
                "no",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn ailoop");

        // Take stdin but do NOT write or close it so the process hits InputResult::Timeout
        let _stdin = child.stdin.take();

        let output = child.wait_with_output().expect("Failed to wait for ailoop");
        drop(_stdin);

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(
            !output.status.success(),
            "authorize --default no should exit non-zero on timeout (InputResult::Timeout path), stdout: {}, stderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("DENIED"),
            "stdout should indicate DENIED on timeout with --default no, got: {}",
            stdout
        );
    }
}
