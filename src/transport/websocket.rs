//! WebSocket transport implementation

use crate::models::Message;
use crate::transport::Transport;
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::SinkExt;
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
