//! WebSocket transport implementation

mod connection;
use connection::{connect_with_retry, parse_websocket_url, ConnectionConfig};

use crate::models::Message;
use crate::transport::Transport;
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_tungstenite::{tungstenite::Message as WsMessage, WebSocketStream};
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
        let url = parse_websocket_url(&self.url)?;
        let config = ConnectionConfig::default();
        let ws_stream = connect_with_retry(&url, &config).await?;
        self.connection = Some(Mutex::new(ws_stream));
        Ok(())
    }

    /// Send buffered messages when connection is restored
    async fn flush_buffer(&mut self) -> Result<()> {
        if self.connection.is_none() && !self.buffer.is_empty() {
            if let Err(e) = self.connect_with_retry().await {
                return Err(anyhow::anyhow!(
                    "Failed to reconnect, {} messages still buffered: {}",
                    self.buffer.len(),
                    e
                ));
            }
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
    let _ = channel;
    let url_parsed = parse_websocket_url(&url)?;

    let (ws_stream, _) = connect_async(url_parsed)
        .await
        .context("Failed to connect to WebSocket server")?;

    let (mut sender, mut receiver) = ws_stream.split();

    let json = serde_json::to_string(&message).context("Failed to serialize message")?;

    sender
        .send(WsMessage::Text(json))
        .await
        .context("Failed to send message")?;

    let timeout_duration = if timeout_secs > 0 {
        tokio::time::Duration::from_secs(timeout_secs as u64)
    } else {
        tokio::time::Duration::from_secs(3600)
    };

    let message_id = message.id;
    let start_time = Instant::now();

    wait_for_response(
        &mut receiver,
        &mut sender,
        message_id,
        timeout_duration,
        start_time,
    )
    .await
}

/// Wait for response message
async fn wait_for_response(
    receiver: &mut futures_util::stream::SplitStream<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    sender: &mut futures_util::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
    message_id: uuid::Uuid,
    timeout_duration: tokio::time::Duration,
    start_time: Instant,
) -> Result<Option<Message>> {
    loop {
        let remaining_time = timeout_duration.saturating_sub(start_time.elapsed());
        if remaining_time.is_zero() {
            return Ok(None);
        }

        tokio::select! {
            msg = receiver.next() => {
                match handle_incoming_message(msg, sender, message_id).await? {
                    MessageHandlerResult::Response(response) => return Ok(Some(response)),
                    MessageHandlerResult::Continue => continue,
                    MessageHandlerResult::None => return Ok(None),
                    MessageHandlerResult::Error(e) => return Err(e),
                }
            }
            _ = tokio::time::sleep(remaining_time) => {
                close_connection_gracefully(sender).await;
                return Ok(None);
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
    message_id: uuid::Uuid,
) -> Result<MessageHandlerResult> {
    match msg {
        Some(Ok(WsMessage::Text(text))) => match serde_json::from_str::<Message>(&text) {
            Ok(message) => {
                if is_response_to_message(&message, message_id) {
                    close_connection_gracefully(sender).await;
                    return Ok(MessageHandlerResult::Response(message));
                }
                Ok(MessageHandlerResult::Continue)
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse message: {}", e);
                Ok(MessageHandlerResult::Continue)
            }
        },
        Some(Ok(WsMessage::Close(_))) => Ok(MessageHandlerResult::None),
        Some(Err(e)) => {
            let _ = sender.close().await;
            Err(anyhow::anyhow!("WebSocket error: {}", e))
        }
        None => {
            let _ = sender.close().await;
            Ok(MessageHandlerResult::None)
        }
        _ => Ok(MessageHandlerResult::Continue),
    }
}

/// Check if message is a response to our message
fn is_response_to_message(message: &Message, message_id: uuid::Uuid) -> bool {
    if let Some(corr_id) = message.correlation_id {
        if corr_id == message_id {
            return true;
        }
    }

    if matches!(
        message.content,
        crate::models::MessageContent::Response { .. }
    ) {
        return true;
    }

    false
}

/// Close WebSocket connection gracefully
async fn close_connection_gracefully(
    sender: &mut futures_util::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
) {
    let close_result = sender.close().await;
    if close_result.is_err() {}
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

/// Result of handling incoming message
enum MessageHandlerResult {
    Response(Message),
    Continue,
    None,
    Error(anyhow::Error),
}

/// Send a message without waiting for response
pub async fn send_message_no_response(
    url: String,
    channel: String,
    message: Message,
) -> Result<()> {
    let _ = channel;
    let url_parsed = parse_websocket_url(&url)?;

    let (ws_stream, _) = connect_async(url_parsed)
        .await
        .context("Failed to connect to WebSocket server")?;

    let (mut sender, _receiver) = ws_stream.split();

    let json = serde_json::to_string(&message).context("Failed to serialize message")?;

    sender
        .send(WsMessage::Text(json))
        .await
        .context("Failed to send message")?;

    let _ = sender.close().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}
