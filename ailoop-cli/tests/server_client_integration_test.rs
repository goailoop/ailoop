//! Integration test for server-client question-answer flow
//!
//! This test starts a server in a background task, sends a question via HTTP API,
//! simulates an answer via HTTP API, and verifies the response is received.

use ailoop_core::models::{Message, MessageContent, SenderType};
use ailoop_core::server::AiloopServer;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_server_client_question_answer() -> Result<()> {
    // Test constants
    const TEST_HOST: &str = "127.0.0.1";
    const TEST_WS_PORT: u16 = 18080;
    const TEST_HTTP_PORT: u16 = 18081;
    const TEST_CHANNEL: &str = "test-channel";
    const TEST_QUESTION: &str = "What is your name?";
    const TEST_ANSWER: &str = "Test Answer";
    const QUESTION_TIMEOUT: u32 = 30; // 30 seconds for question response

    println!("🧪 Starting server-client integration test");

    // 1. Start server in background task
    println!("🚀 Starting server in background...");
    let server = AiloopServer::new(
        TEST_HOST.to_string(),
        TEST_WS_PORT,
        TEST_CHANNEL.to_string(),
    );

    let server_handle = tokio::spawn(async move { server.start().await });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // 2. Wait for server ready
    println!("🏥 Waiting for server to be ready...");
    wait_for_server_ready(TEST_HOST, TEST_WS_PORT, Duration::from_secs(5)).await?;

    // 3. Send question via HTTP API
    println!("❓ Sending question via HTTP API...");
    let question_response = send_question_via_http_api(
        TEST_HOST,
        TEST_HTTP_PORT,
        TEST_QUESTION,
        TEST_CHANNEL,
        QUESTION_TIMEOUT,
    )
    .await?;

    let question_id = question_response["id"]
        .as_str()
        .context("Response should contain message ID")?
        .parse::<uuid::Uuid>()
        .context("Invalid UUID format")?;

    println!("📝 Question sent with ID: {}", question_id);

    // 4. Send response via HTTP API (simulating user answering)
    println!("📤 Simulating user answer via HTTP API...");
    send_answer_via_http_api(TEST_HOST, TEST_HTTP_PORT, TEST_ANSWER, &question_id).await?;

    // 5. Verify response by checking message history
    println!("✅ Verifying response in message history...");
    let messages = get_channel_messages(TEST_HOST, TEST_HTTP_PORT, TEST_CHANNEL).await?;
    println!("📋 Found {} messages in channel", messages.len());

    // Find the response message
    let response_message = messages
        .iter()
        .find(|msg| msg["correlation_id"].as_str() == Some(&question_id.to_string()))
        .context("Response message not found in channel history")?;

    // Check that we got a Response message
    if let Some(content) = response_message["content"].as_object() {
        if content["type"] == "response" {
            if let Some(answer) = content["answer"].as_str() {
                assert_eq!(
                    answer, TEST_ANSWER,
                    "Response answer should match expected answer"
                );
                println!("✅ Received correct answer: {}", answer);
            } else {
                panic!("Response message missing answer field");
            }
        } else {
            panic!("Expected Response message, got type: {:?}", content["type"]);
        }
    } else {
        panic!("Response message missing content");
    }

    // 7. Cleanup - abort server task
    println!("🧹 Cleaning up...");
    server_handle.abort();
    let _ = server_handle.await; // Don't care about result, just wait for abort

    println!("🎉 Test completed successfully!");
    Ok(())
}

/// Wait for server to be ready by trying to connect
async fn wait_for_server_ready(host: &str, port: u16, timeout: Duration) -> Result<()> {
    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Server readiness check timed out after {:?}",
                timeout
            ));
        }

        // Try to connect to server port
        match tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await {
            Ok(_) => {
                println!("✅ Server is listening on port {}", port);
                break;
            }
            Err(_) => {
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        }
    }

    Ok(())
}

/// Send a question via HTTP API
async fn send_question_via_http_api(
    host: &str,
    port: u16,
    question: &str,
    channel: &str,
    timeout: u32,
) -> Result<serde_json::Value> {
    let client = Client::new();
    let url = format!("http://{}:{}/api/v1/messages", host, port);

    // Create a proper Message struct
    let question_content = MessageContent::Question {
        text: question.to_string(),
        timeout_seconds: timeout,
        choices: None,
    };

    let message = Message::new(channel.to_string(), SenderType::Agent, question_content);

    let response = client
        .post(&url)
        .json(&message)
        .send()
        .await
        .context("Failed to send question HTTP request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "HTTP request failed with status {}: {}",
            status,
            body
        ));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse response JSON")?;

    println!("❓ Question sent successfully");
    Ok(response_json)
}

/// Send an answer via HTTP API (simulates user answering the question)
async fn send_answer_via_http_api(
    host: &str,
    port: u16,
    answer: &str,
    message_id: &uuid::Uuid,
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
        .context("Failed to send HTTP request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "HTTP request failed with status {}: {}",
            status,
            body
        ));
    }

    println!(
        "📤 Sent answer '{}' via HTTP API for message {}",
        answer, message_id
    );
    Ok(())
}

/// Get messages from a channel via HTTP API
async fn get_channel_messages(
    host: &str,
    port: u16,
    channel: &str,
) -> Result<Vec<serde_json::Value>> {
    let client = Client::new();
    let url = format!("http://{}:{}/api/channels/{}/messages", host, port, channel);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to get channel messages")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "HTTP request failed with status {}: {}",
            status,
            body
        ));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse response JSON")?;

    let messages = response_json["messages"]
        .as_array()
        .context("Response should contain messages array")?
        .clone();

    Ok(messages)
}
