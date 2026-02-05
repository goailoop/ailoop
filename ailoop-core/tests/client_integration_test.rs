use ailoop_core::client;
use ailoop_core::models::{MessageContent, ResponseType};
use ailoop_core::server::AiloopServer;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::test]
async fn client_ask_returns_server_response() -> Result<()> {
    const HOST: &str = "127.0.0.1";
    const CHANNEL: &str = "integration-channel";
    const QUESTION_TIMEOUT: u32 = 30;
    const ANSWER: &str = "Simulated answer";

    let (ws_port, http_port) = find_free_port_pair(HOST)
        .context("Failed to find free port pair for integration test server")?;
    let server = AiloopServer::new(HOST.to_string(), ws_port, CHANNEL.to_string());
    let server_handle = tokio::spawn(async move { server.start().await });

    wait_for_server_ready(HOST, ws_port, Duration::from_secs(15)).await?;
    wait_for_server_ready(HOST, http_port, Duration::from_secs(15)).await?;

    let server_url = format!("ws://{}:{}", HOST, ws_port);
    let question_text = format!("Test question {}", Uuid::new_v4());
    let question_clone_for_client = question_text.clone();
    let ask_handle = tokio::spawn(async move {
        client::ask(
            &server_url,
            CHANNEL,
            &question_clone_for_client,
            QUESTION_TIMEOUT,
            None,
        )
        .await
    });

    let question_id = wait_for_question_message_id(
        HOST,
        http_port,
        CHANNEL,
        &question_text,
        Duration::from_secs(15),
    )
    .await?;
    send_answer_via_http_api(HOST, http_port, ANSWER, &question_id).await?;

    let response_message = ask_handle
        .await
        .map_err(|e| anyhow::anyhow!("client task panicked: {}", e))??;

    let response_message = response_message.context("Expected server response but got timeout")?;

    if let MessageContent::Response {
        answer,
        response_type,
    } = response_message.content
    {
        assert_eq!(response_type, ResponseType::Text);
        assert_eq!(answer.as_deref(), Some(ANSWER));
    } else {
        panic!(
            "Unexpected response content: {:?}",
            response_message.content
        );
    }

    server_handle.abort();
    let _ = server_handle.await;

    Ok(())
}

fn find_free_port_pair(host: &str) -> Result<(u16, u16)> {
    // The server binds the WebSocket port and (by convention) uses the next port for HTTP.
    // Use OS-assigned ports to avoid collisions when tests run concurrently.
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

async fn wait_for_question_message_id(
    host: &str,
    port: u16,
    channel: &str,
    question_text: &str,
    timeout: Duration,
) -> Result<Uuid> {
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/channels/{}/messages?limit=20",
        host, port, channel
    );
    let start = Instant::now();

    while start.elapsed() < timeout {
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch channel messages")?;

        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse channel messages response")?;

        if let Some(messages) = body["messages"].as_array() {
            for message in messages {
                if message["content"]["type"].as_str() == Some("question")
                    && message["content"]["text"].as_str() == Some(question_text)
                {
                    if let Some(id_str) = message["id"].as_str() {
                        return Uuid::parse_str(id_str)
                            .context("Failed to parse question message ID");
                    }
                }
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow::anyhow!(
        "Timed out waiting for question message '{}' in channel {}",
        question_text,
        channel
    ))
}

async fn send_answer_via_http_api(
    host: &str,
    port: u16,
    answer: &str,
    message_id: &Uuid,
) -> Result<()> {
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/v1/messages/{}/response",
        host, port, message_id
    );
    let response_body = json!({
        "answer": answer,
        "response_type": "text"
    });

    let response = client
        .post(&url)
        .json(&response_body)
        .send()
        .await
        .context("Failed to send answer request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Answer request failed with status {}: {}",
            status,
            body
        ));
    }

    Ok(())
}
