use ailoop_core::client::task_client::TaskClient;
use ailoop_core::models::{DependencyType, TaskState};
use ailoop_core::server::AiloopServer;
use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::time::sleep;

#[tokio::test]
async fn task_client_crud_flow_against_server() -> Result<()> {
    const HOST: &str = "127.0.0.1";
    const CHANNEL: &str = "task-client-channel";

    let (ws_port, http_port) = find_free_port_pair(HOST)
        .context("Failed to find free port pair for task integration server")?;
    let server = AiloopServer::new(HOST.to_string(), ws_port, CHANNEL.to_string());
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server_handle = tokio::spawn(async move {
        server
            .start_with_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
    });

    wait_for_server_ready(HOST, ws_port, Duration::from_secs(5)).await?;
    wait_for_server_ready(HOST, http_port, Duration::from_secs(5)).await?;

    let client = TaskClient::new(format!("http://{}:{}", HOST, http_port));

    let task_a = client
        .create_task("First Task", "Primary", CHANNEL, None, None)
        .await?;
    let task_b = client
        .create_task("Dependent Task", "Depends on first", CHANNEL, None, None)
        .await?;

    let fetched = client.get_task(CHANNEL, &task_a.id.to_string()).await?;
    assert_eq!(fetched.id, task_a.id);

    let tasks = client.list_tasks(CHANNEL, None).await?;
    assert!(tasks.iter().any(|task| task.id == task_a.id));

    client
        .add_dependency(
            CHANNEL,
            &task_b.id.to_string(),
            &task_a.id.to_string(),
            DependencyType::Blocks,
        )
        .await?;

    let blocked = client.list_blocked_tasks(CHANNEL).await?;
    assert!(blocked.iter().any(|task| task.id == task_b.id));

    client
        .update_task_state(CHANNEL, &task_a.id.to_string(), TaskState::Done)
        .await?;

    let done_tasks = client.list_tasks(CHANNEL, Some(TaskState::Done)).await?;
    assert!(done_tasks.iter().any(|task| task.id == task_a.id));

    let ready = client.list_ready_tasks(CHANNEL).await?;
    assert!(ready.iter().any(|task| task.id == task_b.id));

    let graph = client
        .get_dependency_graph(CHANNEL, &task_a.id.to_string())
        .await?;
    let task_a_id = task_a.id.to_string();
    assert_eq!(graph["task"]["id"].as_str(), Some(task_a_id.as_str()));

    client
        .remove_dependency(CHANNEL, &task_b.id.to_string(), &task_a.id.to_string())
        .await?;

    let ready_again = client.list_ready_tasks(CHANNEL).await?;
    assert!(ready_again.iter().any(|task| task.id == task_b.id));

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;

    Ok(())
}

fn find_free_port_pair(host: &str) -> Result<(u16, u16)> {
    for _ in 0..50 {
        let ws_listener = std::net::TcpListener::bind((host, 0))
            .with_context(|| format!("Failed to bind ephemeral port on {}", host))?;
        let ws_port = ws_listener
            .local_addr()
            .context("Failed to get local addr for ws listener")?
            .port();
        drop(ws_listener);

        if ws_port == u16::MAX {
            continue;
        }
        let http_port = ws_port + 1;
        if std::net::TcpListener::bind((host, http_port)).is_ok() {
            return Ok((ws_port, http_port));
        }
    }
    Err(anyhow::anyhow!("Failed to find a free adjacent port pair"))
}

async fn wait_for_server_ready(host: &str, port: u16, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if tcp_connect(host, port).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow::anyhow!(
        "Timed out waiting for server to listen on {}:{}",
        host,
        port
    ))
}

async fn tcp_connect(host: &str, port: u16) -> Result<()> {
    tokio::net::TcpStream::connect(format!("{}:{}", host, port))
        .await
        .context("Failed to connect")
        .map(|_| ())
}
