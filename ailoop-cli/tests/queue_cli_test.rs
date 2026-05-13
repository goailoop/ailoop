//! Integration tests for `ailoop queue` CLI subcommand.

mod common;

use ailoop_server::AiloopServer;
use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::sleep;

const TEST_HOST: &str = "127.0.0.1";

async fn spawn_test_server(
    host: &str,
) -> Result<(u16, oneshot::Sender<()>, JoinHandle<Result<()>>)> {
    let port = common::find_free_port(host).context("Failed to find free port for test server")?;
    let server = AiloopServer::new(host.to_string(), port, "queue-test".to_string());
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server_handle = tokio::spawn(async move {
        server
            .start_with_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
    });
    wait_for_server_ready(host, port, Duration::from_secs(10)).await?;
    Ok((port, shutdown_tx, server_handle))
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
async fn test_queue_json_output_shape() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, port);

    let (ok, stdout, stderr) = run_cmd(&["queue", "--server", &server_url, "--json"]).await;

    assert!(ok, "ailoop queue --json should succeed; stderr: {}", stderr);

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("output should be valid JSON: {}; stdout: {}", e, stdout));

    assert!(
        json.get("items").is_some(),
        "JSON must have 'items' key; got: {}",
        json
    );
    assert!(
        json.get("total_count").is_some(),
        "JSON must have 'total_count' key; got: {}",
        json
    );
    assert_eq!(json["total_count"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_queue_human_readable_header() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, port);

    let (ok, stdout, stderr) = run_cmd(&["queue", "--server", &server_url]).await;

    assert!(ok, "ailoop queue should succeed; stderr: {}", stderr);

    assert!(
        stdout.contains("Human queue:"),
        "output must contain header 'Human queue:'; got: {}",
        stdout
    );
    assert!(
        stdout.contains("pending"),
        "output must contain 'pending'; got: {}",
        stdout
    );

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn test_queue_ailoop_server_env_var() -> Result<()> {
    let _port_lock = common::port_allocation_lock().context("port allocation lock")?;
    let (port, shutdown_tx, server_handle) = spawn_test_server(TEST_HOST).await?;
    let server_url = format!("http://{}:{}", TEST_HOST, port);

    // Use AILOOP_SERVER env var instead of --server flag
    let args: Vec<String> = vec![
        "run".to_string(),
        "--bin".to_string(),
        "ailoop".to_string(),
        "--".to_string(),
        "queue".to_string(),
        "--json".to_string(),
    ];
    let (ok, stdout, stderr) = tokio::task::spawn_blocking(move || {
        let output = Command::new("cargo")
            .args(&args)
            .env("AILOOP_SERVER", &server_url)
            .output()
            .expect("Failed to run ailoop");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )
    })
    .await
    .expect("spawn_blocking panicked");

    assert!(
        ok,
        "ailoop queue with AILOOP_SERVER should succeed; stderr: {}",
        stderr
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("output should be valid JSON: {}; stdout: {}", e, stdout));
    assert!(json.get("items").is_some());
    assert!(json.get("total_count").is_some());

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
    Ok(())
}
