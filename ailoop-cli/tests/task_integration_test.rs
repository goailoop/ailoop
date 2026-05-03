mod common;

use ailoop_core::server::AiloopServer;
use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::sleep;

const TEST_HOST: &str = "127.0.0.1";
const TEST_CHANNEL: &str = "public";

async fn spawn_test_server(
    host: &str,
) -> Result<(u16, oneshot::Sender<()>, JoinHandle<Result<()>>)> {
    let (ws_port, http_port) = common::find_free_adjacent_port_pair(host)
        .context("Failed to find free port pair for test server")?;
    let server = AiloopServer::new(host.to_string(), ws_port, "task-integration".to_string());
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server_handle = tokio::spawn(async move {
        server
            .start_with_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
    });
    wait_for_server_ready(host, http_port, Duration::from_secs(10)).await?;
    Ok((http_port, shutdown_tx, server_handle))
}

async fn wait_for_server_ready(host: &str, port: u16, timeout: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Server readiness check timed out after {:?}",
                timeout
            ));
        }
        if tokio::net::TcpStream::connect(format!("{}:{}", host, port))
            .await
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}

async fn run_cmd(args: &[&str]) -> (bool, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let output = Command::new("cargo")
            .args(["run", "--bin", "ailoop", "--"])
            .args(&args)
            .env_remove("AILOOP_SERVER")
            .env_remove("AILOOP_MODE")
            .output()
            .expect("Failed to run ailoop");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )
    })
    .await
    .expect("spawn_blocking panicked")
}

#[tokio::test]
async fn test_task_create_list_show() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Integration Test Task",
        "--description",
        "A task for integration testing",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create failed: {}", stderr);

    let created: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse task create JSON output")?;
    let task_id = created["id"]
        .as_str()
        .context("Missing id field in create output")?;

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "list",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task list failed: {}", stderr);
    assert!(
        stdout.contains(task_id),
        "task list output should contain created task ID '{}'\nActual: {}",
        task_id,
        stdout
    );

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "show",
        task_id,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task show failed: {}", stderr);
    let shown: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse task show JSON output")?;
    assert_eq!(
        shown["title"].as_str(),
        Some("Integration Test Task"),
        "task show should return correct title"
    );
    assert_eq!(
        shown["state"].as_str(),
        Some("pending"),
        "newly created task should have state pending"
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_update_state() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Update State Task",
        "--description",
        "Task to test state update",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create failed: {}", stderr);

    let created: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse task create JSON output")?;
    let task_id = created["id"].as_str().context("Missing id field")?;

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "update",
        task_id,
        "--state",
        "done",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task update failed: {}", stderr);
    assert!(
        stdout.contains("done"),
        "task update output should contain 'done'\nActual: {}",
        stdout
    );

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "show",
        task_id,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task show failed: {}", stderr);
    let shown: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse task show JSON output")?;
    assert_eq!(
        shown["state"].as_str(),
        Some("done"),
        "task show should return updated state 'done'"
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_ready_and_blocked() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task A (blocker)",
        "--description",
        "This task blocks B",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create A failed: {}", stderr);
    let task_a: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_a = task_a["id"].as_str().context("Missing id for task A")?;

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task B (blocked)",
        "--description",
        "This task is blocked by A",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create B failed: {}", stderr);
    let task_b: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_b = task_b["id"].as_str().context("Missing id for task B")?;

    // B depends on A (B is blocked by A)
    let (ok, _stdout, stderr) = run_cmd(&[
        "task",
        "dep",
        "add",
        id_b,
        id_a,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(ok, "dep add failed: {}", stderr);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "blocked",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task blocked failed: {}", stderr);
    assert!(
        stdout.contains(id_b),
        "blocked output should contain task B id '{}'\nActual: {}",
        id_b,
        stdout
    );

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "ready",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task ready failed: {}", stderr);
    assert!(
        stdout.contains(id_a),
        "ready output should contain task A id '{}'\nActual: {}",
        id_a,
        stdout
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_dep_add_remove_graph() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task C",
        "--description",
        "Child task",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create C failed: {}", stderr);
    let task_c: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_c = task_c["id"].as_str().context("Missing id for task C")?;

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task D",
        "--description",
        "Parent task",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create D failed: {}", stderr);
    let task_d: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_d = task_d["id"].as_str().context("Missing id for task D")?;

    // C depends on D
    let (ok, _stdout, stderr) = run_cmd(&[
        "task",
        "dep",
        "add",
        id_c,
        id_d,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(ok, "dep add failed: {}", stderr);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "dep",
        "graph",
        id_c,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(ok, "dep graph after add failed: {}", stderr);
    assert!(
        stdout.contains("Task D"),
        "dep graph should show Task D as dependency of C\nActual: {}",
        stdout
    );

    let (ok, _stdout, stderr) = run_cmd(&[
        "task",
        "dep",
        "remove",
        id_c,
        id_d,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(ok, "dep remove failed: {}", stderr);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "dep",
        "graph",
        id_c,
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(ok, "dep graph after remove failed: {}", stderr);
    assert!(
        !stdout.contains(id_d),
        "dep graph should not contain task D id after removal\nActual: {}",
        stdout
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_create_json_output() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "JSON Output Task",
        "--description",
        "Testing JSON output fields",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create failed: {}", stderr);

    let task: serde_json::Value =
        serde_json::from_str(&stdout).context("Output should be valid JSON")?;
    assert!(
        task["id"].is_string(),
        "JSON output should contain string 'id' field\nActual: {}",
        stdout
    );
    assert!(
        task["title"].is_string(),
        "JSON output should contain string 'title' field\nActual: {}",
        stdout
    );
    assert!(
        task["state"].is_string(),
        "JSON output should contain string 'state' field\nActual: {}",
        stdout
    );
    assert!(
        task["created_at"].is_string(),
        "JSON output should contain string 'created_at' field\nActual: {}",
        stdout
    );
    assert_eq!(
        task["state"].as_str(),
        Some("pending"),
        "newly created task state should be 'pending'"
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_list_state_filter() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    // Create task E — leave as pending
    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task E (pending)",
        "--description",
        "This task stays pending",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create E failed: {}", stderr);
    let _task_e: serde_json::Value = serde_json::from_str(&stdout)?;

    // Create task F — will be marked done
    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task F (done)",
        "--description",
        "This task will be done",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create F failed: {}", stderr);
    let task_f: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_f = task_f["id"].as_str().context("Missing id for task F")?;

    let (ok, _stdout, stderr) = run_cmd(&[
        "task",
        "update",
        id_f,
        "--state",
        "done",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task update F failed: {}", stderr);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "list",
        "--state",
        "done",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task list --state done failed: {}", stderr);

    let list: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse task list JSON output")?;
    let tasks = list["tasks"]
        .as_array()
        .context("Response should contain tasks array")?;
    for task in tasks {
        assert_eq!(
            task["state"].as_str(),
            Some("done"),
            "all tasks in --state done filter should have state 'done', got: {}",
            task
        );
    }
    assert!(
        tasks.iter().any(|t| t["id"].as_str() == Some(id_f)),
        "done task F should appear in filtered list"
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_update_invalid_state() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Task for invalid state test",
        "--description",
        "Testing invalid state error",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create failed: {}", stderr);
    let created: serde_json::Value = serde_json::from_str(&stdout)?;
    let task_id = created["id"].as_str().context("Missing id field")?;

    let (ok, _stdout, stderr) = run_cmd(&[
        "task",
        "update",
        task_id,
        "--state",
        "invalid_value",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(!ok, "task update with invalid state should fail");
    assert!(
        stderr.contains("Must be pending, done, or abandoned"),
        "stderr should contain 'Must be pending, done, or abandoned'\nActual: {}",
        stderr
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_dep_add_invalid_dependency_type() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Child Task",
        "--description",
        "Child for invalid dep type test",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create child failed: {}", stderr);
    let child: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_child = child["id"].as_str().context("Missing id for child")?;

    let (ok, stdout, stderr) = run_cmd(&[
        "task",
        "create",
        "Parent Task",
        "--description",
        "Parent for invalid dep type test",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(ok, "task create parent failed: {}", stderr);
    let parent: serde_json::Value = serde_json::from_str(&stdout)?;
    let id_parent = parent["id"].as_str().context("Missing id for parent")?;

    let (ok, _stdout, stderr) = run_cmd(&[
        "task",
        "dep",
        "add",
        id_child,
        id_parent,
        "--dependency-type",
        "invalid",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
    ])
    .await;
    assert!(!ok, "dep add with invalid dependency type should fail");
    assert!(
        stderr.contains("Must be blocks, related, or parent"),
        "stderr should contain 'Must be blocks, related, or parent'\nActual: {}",
        stderr
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_task_show_nonexistent_id() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (http_port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, http_port);

    let (ok, _stdout, _stderr) = run_cmd(&[
        "task",
        "show",
        "00000000-0000-0000-0000-000000000000",
        "--channel",
        TEST_CHANNEL,
        "--server",
        &server_url,
        "--json",
    ])
    .await;
    assert!(
        !ok,
        "task show with nonexistent UUID should exit non-zero (HTTP 404)"
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}
