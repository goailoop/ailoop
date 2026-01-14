//! WebSocket transport implementation

use crate::models::Message;
use crate::transport::Transport;
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage, WebSocketStream};
use url::Url;

/// WebSocket transport for sending messages to ailoop server
pub struct WebSocketTransport {
    url: String,
    channel: String,
    client_id: Option<String>,
    connection: Option<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
    buffer: VecDeque<Message>,
    max_buffer_size: usize,
}


impl WebSocketTransport {
    /// Create a new WebSocket transport
    pub fn new(url: String, channel: String, client_id: Option<String>) -> Result<Self> {
        Ok(Self {
            url,
            channel,
            client_id,
            connection: None,
            buffer: VecDeque::new(),
            max_buffer_size: 1000,
        })
    }

    /// Connect to the WebSocket server with retry logic and exponential backoff
    async fn connect_with_retry(&mut self) -> Result<()> {
        const MAX_RETRIES: u32 = 5;
        const INITIAL_DELAY_MS: u64 = 100;
        const MAX_DELAY_MS: u64 = 10000;
        const MAX_TIMEOUT_MS: u64 = 30000; // 30 seconds total timeout

        let url = Url::parse(&self.url)
            .with_context(|| format!("Invalid WebSocket URL: {}", self.url))?;

        let start_time = std::time::Instant::now();
        let mut delay = INITIAL_DELAY_MS;

        for attempt in 0..MAX_RETRIES {
            // Check if we've exceeded the maximum timeout
            if start_time.elapsed().as_millis() as u64 > MAX_TIMEOUT_MS {
                return Err(anyhow::anyhow!(
                    "Connection timeout: Failed to connect within {}ms",
                    MAX_TIMEOUT_MS
                ));
            }

            // Try to connect
            match connect_async(url.clone()).await {
                Ok((ws_stream, _)) => {
                    self.connection = Some(Mutex::new(ws_stream));
                    return Ok(());
                }
                Err(e) => {
                    if attempt == MAX_RETRIES - 1 {
                        // Last attempt failed
                        return Err(anyhow::anyhow!(
                            "Failed to connect after {} attempts: {}",
                            MAX_RETRIES,
                            e
                        ));
                    }
                    // Wait with exponential backoff before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    delay = std::cmp::min(delay * 2, MAX_DELAY_MS);
                }
            }
        }

        Err(anyhow::anyhow!("Failed to connect to WebSocket: {}", self.url))
    }

    /// Send buffered messages when connection is restored
    async fn flush_buffer(&mut self) -> Result<()> {
        // Try to reconnect if not connected
        if self.connection.is_none() && !self.buffer.is_empty() {
            if let Err(e) = self.connect_with_retry().await {
                // If reconnection fails, keep messages in buffer
                return Err(anyhow::anyhow!(
                    "Failed to reconnect, {} messages still buffered: {}",
                    self.buffer.len(),
                    e
                ));
            }
        }

        // Send all buffered messages
        while let Some(message) = self.buffer.pop_front() {
            match self.send_internal(message.clone()).await {
                Ok(()) => {
                    // Message sent successfully, continue
                }
                Err(e) => {
                    // Connection lost again, put message back and stop
                    self.connection = None;
                    self.buffer.push_front(message);
                    return Err(anyhow::anyhow!(
                        "Connection lost while flushing buffer: {}",
                        e
                    ));
                }
            }
        }

        Ok(())
    }

    /// Internal send method
    async fn send_internal(&mut self, message: Message) -> Result<()> {
        if self.connection.is_none() {
            self.connect_with_retry().await?;
        }

        let conn = self.connection.as_mut().unwrap();
        let mut stream = conn.lock().await;

        let json = serde_json::to_string(&message)
            .context("Failed to serialize message")?;

        stream
            .send(WsMessage::Text(json))
            .await
            .context("Failed to send message over WebSocket")?;

        Ok(())
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send(&mut self, message: Message) -> Result<()> {
        if self.connection.is_none() {
            // Try to connect, but buffer if connection fails
            if let Err(e) = self.connect_with_retry().await {
                // Buffer message for later delivery
                if self.buffer.len() >= self.max_buffer_size {
                    self.buffer.pop_front(); // FIFO eviction
                }
                self.buffer.push_back(message);
                return Err(e);
            }
        }

        // Try to send, buffer if connection lost
        match self.send_internal(message.clone()).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Connection lost, buffer message
                self.connection = None;
                if self.buffer.len() >= self.max_buffer_size {
                    self.buffer.pop_front();
                }
                self.buffer.push_back(message);
                Err(e)
            }
        }
    }

    async fn flush(&mut self) -> Result<()> {
        self.flush_buffer().await
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(conn) = self.connection.take() {
            let mut stream = conn.lock().await;
            stream.close(None).await.context("Failed to close WebSocket")?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "websocket"
    }
}

/// Send a message and wait for response (common function for ask/authorize commands)
pub async fn send_message_and_wait_response(
    url: String,
    channel: String,
    message: Message,
    timeout_secs: u32,
) -> Result<Option<Message>> {
    use futures_util::{SinkExt, StreamExt};
    
    // Connect to WebSocket
    let url_parsed = Url::parse(&url)
        .with_context(|| format!("Invalid WebSocket URL: {}", url))?;
    
    let (ws_stream, _) = connect_async(url_parsed).await
        .context("Failed to connect to WebSocket server")?;
    
    // Split into sender and receiver
    let (mut sender, mut receiver) = ws_stream.split();
    
    // Send the message
    let json = serde_json::to_string(&message)
        .context("Failed to serialize message")?;
    
    sender.send(WsMessage::Text(json)).await
        .context("Failed to send message")?;
    
    // Wait for response with timeout
    let timeout_duration = if timeout_secs > 0 {
        tokio::time::Duration::from_secs(timeout_secs as u64)
    } else {
        tokio::time::Duration::from_secs(3600) // 1 hour default
    };
    
    let message_id = message.id;
    let start_time = std::time::Instant::now();
    
    // Keep receiving messages until we find the response or timeout
    loop {
        let remaining_time = timeout_duration.saturating_sub(start_time.elapsed());
        if remaining_time.is_zero() {
            return Ok(None); // Timeout
        }
        
        let result = tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        match serde_json::from_str::<Message>(&text) {
                            Ok(message) => {
                                // Check if this is a response to our message
                                if let Some(corr_id) = message.correlation_id {
                                    if corr_id == message_id {
                                        // Found our response - close connection gracefully
                                        // Send close frame and wait a bit for it to be processed
                                        let close_result = sender.close().await;
                                        if close_result.is_err() {
                                            // Connection might already be closed, that's okay
                                        }
                                        // Give time for close handshake
                                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                        return Ok(Some(message));
                                    }
                                    // Not our response, continue waiting
                                    continue;
                                }
                                // Check if message content is a Response type
                                if matches!(message.content, crate::models::MessageContent::Response { .. }) {
                                    // Assume it's our response if it's a Response type
                                    // Close connection gracefully
                                    let close_result = sender.close().await;
                                    if close_result.is_err() {
                                        // Connection might already be closed, that's okay
                                    }
                                    // Give time for close handshake
                                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                    return Ok(Some(message));
                                }
                                // Not a response, continue waiting
                                continue;
                            }
                            Err(e) => {
                                // Failed to parse, continue waiting
                                eprintln!("Warning: Failed to parse message: {}", e);
                                continue;
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        // Server closed connection, that's fine
                        return Ok(None);
                    }
                    Some(Err(e)) => {
                        // Error occurred, try to close gracefully
                        let _ = sender.close().await;
                        return Err(anyhow::anyhow!("WebSocket error: {}", e));
                    }
                    None => {
                        // Stream ended, close gracefully
                        let _ = sender.close().await;
                        return Ok(None);
                    }
                    _ => {
                        continue;
                    }
                }
            }
            _ = tokio::time::sleep(remaining_time) => {
                // Timeout - close connection gracefully
                let _ = sender.close().await;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                return Ok(None);
            }
        };
    }
    
    // This should never be reached, but if it is, close the connection
    let _ = sender.close().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    Ok(None)
}

/// Send a message without waiting for response (for one-way messages like navigate)
pub async fn send_message_no_response(
    url: String,
    channel: String,
    message: Message,
) -> Result<()> {
    use futures_util::SinkExt;
    
    // Connect to WebSocket
    let url_parsed = Url::parse(&url)
        .with_context(|| format!("Invalid WebSocket URL: {}", url))?;
    
    let (ws_stream, _) = connect_async(url_parsed).await
        .context("Failed to connect to WebSocket server")?;
    
    // Split into sender and receiver
    let (mut sender, _receiver) = ws_stream.split();
    
    // Send the message
    let json = serde_json::to_string(&message)
        .context("Failed to serialize message")?;
    
    sender.send(WsMessage::Text(json)).await
        .context("Failed to send message")?;
    
    // Close the connection gracefully
    let _ = sender.close().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    Ok(())
}
