use ailoop_core::client;
use ailoop_core::models::{MessageContent, ResponseType};
use ailoop_core::server::AiloopServer;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::connect_async;
use uuid::Uuid;

struct TestServer {
    shutdown_tx: oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<Result<()>>,
}

#[tokio::test]
async fn client_ask_returns_server_response() -> Result<()> {
    const HOST: &str = "127.0.0.1";
    const CHANNEL: &str = "integration-channel";
    const QUESTION_TIMEOUT: u32 = 30;
    const ANSWER: &str = "Simulated answer";

    let (ws_port, http_port) = find_free_port_pair(HOST)
        .context("Failed to find free port pair for integration test server")?;
    let server = start_test_server(HOST, ws_port, CHANNEL)?;
    wait_for_http_ready(HOST, http_port, Duration::from_secs(15)).await?;
    wait_for_ws_ready(HOST, ws_port, Duration::from_secs(15)).await?;

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
    send_response_via_http_api(HOST, http_port, Some(ANSWER), "text", &question_id).await?;

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

    let _ = server.shutdown_tx.send(());
    let _ = server.handle.await;

    Ok(())
}

#[tokio::test]
async fn client_authorize_returns_server_response() -> Result<()> {
    const HOST: &str = "127.0.0.1";
    const CHANNEL: &str = "integration-auth-channel";
    const TIMEOUT_SECS: u32 = 30;
    const ACTION: &str = "Deploy to prod?";

    let (ws_port, http_port) = find_free_port_pair(HOST)
        .context("Failed to find free port pair for integration test server")?;
    let server = start_test_server(HOST, ws_port, CHANNEL)?;

    wait_for_http_ready(HOST, http_port, Duration::from_secs(15)).await?;
    wait_for_ws_ready(HOST, ws_port, Duration::from_secs(15)).await?;

    let server_url = format!("ws://{}:{}", HOST, ws_port);
    let authorize_handle =
        tokio::spawn(
            async move { client::authorize(&server_url, CHANNEL, ACTION, TIMEOUT_SECS).await },
        );

    let msg_id = wait_for_interactive_message_id(
        HOST,
        http_port,
        CHANNEL,
        "authorization",
        "action",
        ACTION,
        Duration::from_secs(15),
    )
    .await?;

    send_response_via_http_api(HOST, http_port, None, "authorization_approved", &msg_id).await?;

    let response_message = authorize_handle
        .await
        .map_err(|e| anyhow::anyhow!("client task panicked: {}", e))??;

    let response_message = response_message.context("Expected server response but got timeout")?;

    if let MessageContent::Response { response_type, .. } = response_message.content {
        assert_eq!(response_type, ResponseType::AuthorizationApproved);
    } else {
        panic!(
            "Unexpected response content: {:?}",
            response_message.content
        );
    }

    let _ = server.shutdown_tx.send(());
    let _ = server.handle.await;

    Ok(())
}

fn start_test_server(host: &str, ws_port: u16, channel: &str) -> Result<TestServer> {
    let http_port = ws_port
        .checked_add(1)
        .context("Failed to compute HTTP port for test server")?;

    eprintln!(
        "Starting test server: ws_port={}, http_port={}, channel={}",
        ws_port, http_port, channel
    );

    let server = AiloopServer::new(host.to_string(), ws_port, channel.to_string());
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        match server
            .start_with_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
        {
            Ok(()) => {
                eprintln!("Test server shut down cleanly");
                Ok(())
            }
            Err(e) => {
                eprintln!("Test server error: {:?}", e);
                Err(e)
            }
        }
    });

    Ok(TestServer {
        shutdown_tx,
        handle: server_handle,
    })
}

fn find_free_port_pair(host: &str) -> Result<(u16, u16)> {
    // The server binds the WebSocket port and (by convention) uses the next port for HTTP.
    // Use OS-assigned ports to avoid collisions when tests run concurrently.
    //
    // Strategy: Find a port range where both ws_port and ws_port+1 are free.
    // To minimize race conditions, we retry with delays and verify both ports
    // are available before returning.
    for attempt in 0..100 {
        let ws_listener = std::net::TcpListener::bind((host, 0)).with_context(|| {
            format!(
                "Failed to bind ephemeral port on {} (attempt {})",
                host,
                attempt + 1
            )
        })?;
        let ws_port = ws_listener
            .local_addr()
            .context("Failed to get local addr for ws listener")?
            .port();

        if ws_port == u16::MAX {
            drop(ws_listener);
            continue;
        }

        let http_port = ws_port + 1;

        // Check if HTTP port is available
        match std::net::TcpListener::bind((host, http_port)) {
            Ok(http_listener) => {
                // Success! Both ports are available.
                // Hold both listeners for a moment to reserve the ports,
                // then drop them just before returning so the server can bind.
                let ports = (ws_port, http_port);

                // Drop in reverse order to minimize race window
                drop(http_listener);
                drop(ws_listener);

                return Ok(ports);
            }
            Err(e) => {
                // HTTP port not available, try again
                drop(ws_listener);

                // Add exponential backoff for retries
                if attempt < 99 {
                    let delay_ms = std::cmp::min(100, 10 * (1 << std::cmp::min(attempt / 10, 3)));
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                }

                // Log every 20 attempts to help debug if this becomes a persistent issue
                if (attempt + 1) % 20 == 0 {
                    eprintln!(
                        "Warning: find_free_port_pair attempt {} failed: HTTP port unavailable (last error: {})",
                        attempt + 1,
                        e
                    );
                }
                continue;
            }
        }
    }
    Err(anyhow::anyhow!(
        "Failed to find a free adjacent port pair after 100 attempts. \
         This suggests severe port exhaustion or a system issue."
    ))
}

async fn wait_for_http_ready(host: &str, port: u16, deadline: Duration) -> Result<()> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(2))
        .build()
        .context("Failed to build HTTP client")?;
    let url = format!("http://{}:{}/api/v1/health", host, port);
    let start = Instant::now();
    let mut last_error: Option<String> = None;

    while start.elapsed() < deadline {
        match timeout(Duration::from_secs(2), client.get(&url).send()).await {
            Ok(Ok(resp)) => {
                if resp.status().is_success() {
                    eprintln!("HTTP server ready on {}:{}", host, port);
                    return Ok(());
                } else {
                    last_error = Some(format!("HTTP status: {}", resp.status()));
                }
            }
            Ok(Err(e)) => {
                last_error = Some(format!("Request error: {}", e));
            }
            Err(_) => {
                last_error = Some("Request timeout".to_string());
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow::anyhow!(
        "Timed out waiting for HTTP server health on {}:{}. Last error: {}",
        host,
        port,
        last_error.unwrap_or_else(|| "No error captured".to_string())
    ))
}

async fn wait_for_ws_ready(host: &str, port: u16, deadline: Duration) -> Result<()> {
    let url = format!("ws://{}:{}", host, port);
    let start = Instant::now();
    let mut last_error: Option<String> = None;

    while start.elapsed() < deadline {
        match timeout(Duration::from_secs(2), connect_async(&url)).await {
            Ok(Ok(_)) => {
                eprintln!("WebSocket server ready on {}:{}", host, port);
                return Ok(());
            }
            Ok(Err(e)) => {
                last_error = Some(format!("Connection error: {}", e));
            }
            Err(_) => {
                last_error = Some("Connection timeout".to_string());
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow::anyhow!(
        "Timed out waiting for WebSocket handshake on {}:{}. Last error: {}",
        host,
        port,
        last_error.unwrap_or_else(|| "No error captured".to_string())
    ))
}

async fn wait_for_question_message_id(
    host: &str,
    port: u16,
    channel: &str,
    question_text: &str,
    timeout: Duration,
) -> Result<Uuid> {
    wait_for_interactive_message_id(
        host,
        port,
        channel,
        "question",
        "text",
        question_text,
        timeout,
    )
    .await
}

async fn wait_for_interactive_message_id(
    host: &str,
    port: u16,
    channel: &str,
    content_type: &str,
    match_field: &str,
    match_value: &str,
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
                if message["content"]["type"].as_str() == Some(content_type)
                    && message["content"][match_field].as_str() == Some(match_value)
                {
                    if let Some(id_str) = message["id"].as_str() {
                        return Uuid::parse_str(id_str)
                            .context("Failed to parse interactive message ID");
                    }
                }
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow::anyhow!(
        "Timed out waiting for {} message '{}' in channel {}",
        content_type,
        match_value,
        channel
    ))
}

async fn send_response_via_http_api(
    host: &str,
    port: u16,
    answer: Option<&str>,
    response_type: &str,
    message_id: &Uuid,
) -> Result<()> {
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/v1/messages/{}/response",
        host, port, message_id
    );
    let response_body = json!({
        "answer": answer,
        "response_type": response_type
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
