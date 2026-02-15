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
    connection:
        Option<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
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
        const MAX_TIMEOUT_MS: u64 = 30000;

        let url = Url::parse(&self.url)
            .with_context(|| format!("Invalid WebSocket URL: {}", self.url))?;

        let start_time = std::time::Instant::now();
        let mut delay = INITIAL_DELAY_MS;

        for attempt in 0..MAX_RETRIES {
            if start_time.elapsed().as_millis() as u64 > MAX_TIMEOUT_MS {
                return Err(anyhow::anyhow!(
                    "Connection timeout: Failed to connect within {}ms",
                    MAX_TIMEOUT_MS
                ));
            }

            match connect_async(url.clone()).await {
                Ok((ws_stream, _)) => {
                    self.connection = Some(Mutex::new(ws_stream));
                    return Ok(());
                }
                Err(e) => {
                    if attempt == MAX_RETRIES - 1 {
                        return Err(anyhow::anyhow!(
                            "Failed to connect after {} attempts: {}",
                            MAX_RETRIES,
                            e
                        ));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    delay = std::cmp::min(delay * 2, MAX_DELAY_MS);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to connect to WebSocket: {}",
            self.url
        ))
    }

    /// Send buffered messages when connection is restored
    async fn flush_buffer(&mut self) -> Result<()> {
        if self.connection.is_none() && !self.buffer.is_empty() {
            self.connect_with_retry().await?;
        }

        while let Some(message) = self.buffer.pop_front() {
            match self.send_internal(message.clone()).await {
                Ok(()) => {}
                Err(e) => {
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

        let json = serde_json::to_string(&message).context("Failed to serialize message")?;

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
            if let Err(e) = self.connect_with_retry().await {
                if self.buffer.len() >= self.max_buffer_size {
                    self.buffer.pop_front();
                }
                self.buffer.push_back(message);
                return Err(e);
            }
        }

        match self.send_internal(message.clone()).await {
            Ok(()) => Ok(()),
            Err(e) => {
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
            stream
                .close(None)
                .await
                .context("Failed to close WebSocket")?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "websocket"
    }
}

/// Send a message and wait for response
pub async fn send_message_and_wait_response(
    url: String,
    channel: String,
    message: Message,
    timeout_secs: u32,
) -> Result<Option<Message>> {
    let url_parsed = Url::parse(&url).with_context(|| format!("Invalid WebSocket URL: {}", url))?;

    let (ws_stream, _) = connect_async(url_parsed)
        .await
        .context("Failed to connect to WebSocket server")?;

    let (mut sender, mut receiver) = ws_stream.split();

    send_message(&mut sender, &message).await?;

    let timeout_duration = calculate_timeout(timeout_secs);
    let message_id = message.id;
    let start_time = std::time::Instant::now();

    wait_for_response(
        &mut receiver,
        &mut sender,
        message_id,
        timeout_duration,
        start_time,
    )
    .await
}

/// Send message through WebSocket
async fn send_message(
    sender: &mut futures_util::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
    message: &Message,
) -> Result<()> {
    let json = serde_json::to_string(message).context("Failed to serialize message")?;

    sender
        .send(WsMessage::Text(json))
        .await
        .context("Failed to send message")?;

    Ok(())
}

/// Calculate timeout duration
fn calculate_timeout(timeout_secs: u32) -> tokio::time::Duration {
    if timeout_secs > 0 {
        tokio::time::Duration::from_secs(timeout_secs as u64)
    } else {
        tokio::time::Duration::from_secs(3600)
    }
}

/// Wait for response from WebSocket
async fn wait_for_response(
    receiver: &mut futures_util::stream::SplitStream<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    sender: &mut futures_util::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
    message_id: String,
    timeout_duration: tokio::time::Duration,
    start_time: std::time::Instant,
) -> Result<Option<Message>> {
    loop {
        let remaining_time = timeout_duration.saturating_sub(start_time.elapsed());
        if remaining_time.is_zero() {
            return close_connection_gracefully(sender, None);
        }

        tokio::select! {
            msg = receiver.next() => {
                match handle_incoming_message(msg, sender, &message_id).await? {
                    Some(response) => return Ok(Some(response)),
                    None => continue,
                }
            }
            _ = tokio::time::sleep(remaining_time) => {
                return close_connection_gracefully(sender, None);
            }
        };
    }
}

/// Handle incoming message from WebSocket
async fn handle_incoming_message(
    msg: Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>>,
    sender: &mut futures_util::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
    message_id: &str,
) -> Result<Option<Message>> {
    match msg {
        Some(Ok(WsMessage::Text(text))) => match serde_json::from_str::<Message>(&text) {
            Ok(message) => {
                if is_our_response(&message, message_id) {
                    close_connection_gracefully(sender, Some(message.clone())).await?;
                    return Ok(Some(message));
                }
                Ok(None)
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse message: {}", e);
                Ok(None)
            }
        },
        Some(Ok(WsMessage::Close(_))) => Ok(None),
        Some(Err(e)) => {
            close_connection_gracefully(sender, None).await?;
            Err(anyhow::anyhow!("WebSocket error: {}", e))
        }
        None => {
            close_connection_gracefully(sender, None).await?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Check if message is a response to our request
fn is_our_response(message: &Message, message_id: &str) -> bool {
    if let Some(corr_id) = &message.correlation_id {
        if corr_id == message_id {
            return true;
        }
    }

    matches!(
        message.content,
        crate::models::MessageContent::Response { .. }
    )
}

/// Close connection gracefully
async fn close_connection_gracefully<T>(
    sender: &mut futures_util::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
    response: Option<T>,
) -> Result<Option<T>> {
    let _ = sender.close().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    Ok(response)
}

/// Send a message without waiting for response
pub async fn send_message_no_response(
    url: String,
    channel: String,
    message: Message,
) -> Result<()> {
    let url_parsed = Url::parse(&url).with_context(|| format!("Invalid WebSocket URL: {}", url))?;

    let (ws_stream, _) = connect_async(url_parsed)
        .await
        .context("Failed to connect to WebSocket server")?;

    let (mut sender, _receiver) = ws_stream.split();

    send_message(&mut sender, &message).await?;

    close_connection_gracefully(&mut sender, ()).await?;

    Ok(())
}
