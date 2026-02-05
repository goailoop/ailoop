use ailoop_core::client::task_client::TaskClient;
use ailoop_core::models::{DependencyType, TaskState};
use ailoop_core::server::AiloopServer;
use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn task_client_crud_flow_against_server() -> Result<()> {
    const HOST: &str = "127.0.0.1";
    const WS_PORT: u16 = 18380;
    const HTTP_PORT: u16 = 18381;
    const CHANNEL: &str = "task-client-channel";

    let server = AiloopServer::new(HOST.to_string(), WS_PORT, CHANNEL.to_string());
    let server_handle = tokio::spawn(async move { server.start().await });

    wait_for_server_ready(HOST, WS_PORT, Duration::from_secs(5)).await?;
    wait_for_server_ready(HOST, HTTP_PORT, Duration::from_secs(5)).await?;

    let client = TaskClient::new(&format!("http://{}:{}", HOST, HTTP_PORT));

    let task_a = client
        .create_task("First Task", "Primary", CHANNEL, None, None)
        .await?;
    let task_b = client
        .create_task("Dependent Task", "Depends on first", CHANNEL, None, None)
        .await?;

    let fetched = client.get_task(&task_a.id.to_string()).await?;
    assert_eq!(fetched.id, task_a.id);

    let tasks = client.list_tasks(CHANNEL, None).await?;
    assert!(tasks.iter().any(|task| task.id == task_a.id));

    client
        .add_dependency(
            &task_b.id.to_string(),
            &task_a.id.to_string(),
            DependencyType::Blocks,
        )
        .await?;

    let blocked = client.list_blocked_tasks(CHANNEL).await?;
    assert!(blocked.iter().any(|task| task.id == task_b.id));

    client
        .update_task_state(&task_a.id.to_string(), TaskState::Done)
        .await?;

    let done_tasks = client.list_tasks(CHANNEL, Some(TaskState::Done)).await?;
    assert!(done_tasks.iter().any(|task| task.id == task_a.id));

    let ready = client.list_ready_tasks(CHANNEL).await?;
    assert!(ready.iter().any(|task| task.id == task_b.id));

    let graph = client.get_dependency_graph(&task_a.id.to_string()).await?;
    assert_eq!(graph["task"]["id"].as_str(), Some(&task_a.id.to_string()));

    client
        .remove_dependency(&task_b.id.to_string(), &task_a.id.to_string())
        .await?;

    let ready_again = client.list_ready_tasks(CHANNEL).await?;
    assert!(ready_again.iter().any(|task| task.id == task_b.id));

    server_handle.abort();
    let _ = server_handle.await;

    Ok(())
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
