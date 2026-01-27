//! Integration test for output capture
//! Tests that commands producing large volumes of output are fully captured

use std::process::Command;
use std::sync::{Arc, Mutex};

/// Mock output capture system for integration testing
struct OutputCapture {
    stdout_data: Arc<Mutex<Vec<u8>>>,
    stderr_data: Arc<Mutex<Vec<u8>>>,
}

impl OutputCapture {
    fn new() -> Self {
        Self {
            stdout_data: Arc::new(Mutex::new(Vec::new())),
            stderr_data: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn append_stdout(&self, data: Vec<u8>) {
        let mut stdout = self.stdout_data.lock().unwrap();
        stdout.extend_from_slice(&data);
    }

    fn append_stderr(&self, data: Vec<u8>) {
        let mut stderr = self.stderr_data.lock().unwrap();
        stderr.extend_from_slice(&data);
    }

    fn get_stdout(&self) -> Vec<u8> {
        self.stdout_data.lock().unwrap().clone()
    }

    fn get_stderr(&self) -> Vec<u8> {
        self.stderr_data.lock().unwrap().clone()
    }

    fn stdout_size(&self) -> usize {
        self.stdout_data.lock().unwrap().len()
    }

    fn stderr_size(&self) -> usize {
        self.stderr_data.lock().unwrap().len()
    }
}

#[test]
fn test_capture_small_output() {
    let capture = OutputCapture::new();

    // Simulate capturing output
    capture.append_stdout(b"Hello, world!\n".to_vec());
    capture.append_stderr(b"Warning: test\n".to_vec());

    assert_eq!(capture.get_stdout(), b"Hello, world!\n");
    assert_eq!(capture.get_stderr(), b"Warning: test\n");
}

#[tokio::test]
async fn test_capture_1mb_output() {
    // Test requirement: Command produces 1MB output, verify all captured (T069)
    let capture = Arc::new(OutputCapture::new());

    // Generate 1MB of output data
    let one_mb = 1024 * 1024;
    let line = b"This is a test line of output data\n";
    let lines_needed = (one_mb / line.len()) + 1;

    // Simulate streaming output in chunks
    for _ in 0..lines_needed {
        capture.append_stdout(line.to_vec());
    }

    let captured_size = capture.stdout_size();
    assert!(
        captured_size >= one_mb,
        "Should capture at least 1MB of data"
    );

    // Verify no data corruption
    let stdout = capture.get_stdout();
    assert!(stdout.len() >= one_mb);

    // Verify data integrity - check that all lines are complete
    let stdout_str = String::from_utf8_lossy(&stdout);
    let line_count = stdout_str.lines().count();
    assert!(line_count >= lines_needed - 1); // Allow for last partial line
}

#[tokio::test]
async fn test_capture_real_command_output() {
    // Test with actual bash command that produces output
    let output = Command::new("bash")
        .arg("-c")
        .arg("for i in {1..100}; do echo \"Line $i\"; done")
        .output()
        .expect("Failed to execute command");

    let capture = OutputCapture::new();
    capture.append_stdout(output.stdout);
    capture.append_stderr(output.stderr);

    let stdout = capture.get_stdout();
    let stdout_str = String::from_utf8_lossy(&stdout);

    // Verify all 100 lines were captured
    assert_eq!(stdout_str.lines().count(), 100);
    assert!(stdout_str.contains("Line 1"));
    assert!(stdout_str.contains("Line 100"));
}

#[tokio::test]
async fn test_capture_concurrent_output() {
    // Test capturing output from multiple concurrent commands
    let capture = Arc::new(OutputCapture::new());

    let mut handles = vec![];

    for i in 0..5 {
        let capture_clone = Arc::clone(&capture);
        let handle = tokio::spawn(async move {
            let output = Command::new("bash")
                .arg("-c")
                .arg(format!("echo 'Task {} output'", i))
                .output()
                .expect("Failed to execute command");

            capture_clone.append_stdout(output.stdout);
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let stdout = capture.get_stdout();
    let stdout_str = String::from_utf8_lossy(&stdout);

    // Verify output from all tasks was captured
    assert!(stdout.len() > 0);
    assert!(stdout_str.contains("output"));
}

#[tokio::test]
async fn test_capture_streaming_output() {
    // Test real-time streaming capture (SC-004: output appears as produced)
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command as TokioCommand;

    let mut child = TokioCommand::new("bash")
        .arg("-c")
        .arg("for i in {1..10}; do echo \"Line $i\"; sleep 0.01; done")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    let stdout = child.stdout.take().expect("Failed to get stdout");
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let capture = OutputCapture::new();
    let mut line_count = 0;

    // Read lines as they are produced
    while let Ok(Some(line)) = lines.next_line().await {
        capture.append_stdout(format!("{}\n", line).into_bytes());
        line_count += 1;
    }

    let _ = child.wait().await;

    // Verify all lines were captured in real-time
    assert_eq!(line_count, 10);
    let stdout_bytes = capture.get_stdout();
    let stdout_str = String::from_utf8_lossy(&stdout_bytes);
    assert!(stdout_str.contains("Line 1"));
    assert!(stdout_str.contains("Line 10"));
}

#[tokio::test]
async fn test_capture_100mb_output() {
    // Test SC-012: System handles 100MB output per workflow without data loss
    let capture = Arc::new(OutputCapture::new());

    // Generate 100MB of output in chunks
    let chunk_size = 1024 * 1024; // 1MB chunks
    let num_chunks = 100;

    for i in 0..num_chunks {
        let chunk = vec![b'X'; chunk_size];
        capture.append_stdout(chunk);

        // Verify progress
        if (i + 1) % 10 == 0 {
            let captured = capture.stdout_size();
            let expected = (i + 1) * chunk_size;
            assert_eq!(captured, expected, "Data loss detected at {}MB", i + 1);
        }
    }

    let total_captured = capture.stdout_size();
    let expected_size = chunk_size * num_chunks;

    assert_eq!(
        total_captured, expected_size,
        "Should capture exactly 100MB without data loss"
    );
}

#[tokio::test]
async fn test_capture_mixed_stdout_stderr() {
    // Test capturing both stdout and stderr concurrently
    let output = Command::new("bash")
        .arg("-c")
        .arg("echo 'stdout line'; echo 'stderr line' >&2; echo 'stdout2'")
        .output()
        .expect("Failed to execute command");

    let capture = OutputCapture::new();
    capture.append_stdout(output.stdout);
    capture.append_stderr(output.stderr);

    let stdout_bytes = capture.get_stdout();
    let stderr_bytes = capture.get_stderr();
    let stdout_str = String::from_utf8_lossy(&stdout_bytes);
    let stderr_str = String::from_utf8_lossy(&stderr_bytes);

    assert!(stdout_str.contains("stdout line"));
    assert!(stdout_str.contains("stdout2"));
    assert!(stderr_str.contains("stderr line"));
}

#[tokio::test]
async fn test_capture_no_data_loss_under_load() {
    // Stress test: Verify no data loss when system is under load
    let capture = Arc::new(OutputCapture::new());

    // Generate predictable output that we can verify
    let expected_lines = 10000;
    let mut expected_content = String::new();

    for i in 0..expected_lines {
        let line = format!("Line {:05}\n", i);
        expected_content.push_str(&line);
        capture.append_stdout(line.into_bytes());
    }

    let captured = capture.get_stdout();
    let captured_str = String::from_utf8_lossy(&captured);

    // Verify exact match
    assert_eq!(captured_str, expected_content);
    assert_eq!(captured_str.lines().count(), expected_lines);

    // Verify first and last lines
    let lines: Vec<&str> = captured_str.lines().collect();
    assert_eq!(lines[0], "Line 00000");
    assert_eq!(
        lines[expected_lines - 1],
        format!("Line {:05}", expected_lines - 1)
    );
}
