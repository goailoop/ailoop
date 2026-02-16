//! WebSocket connection utilities

use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::{connect_async, WebSocketStream};
use url::Url;

/// WebSocket connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub max_timeout_ms: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            max_timeout_ms: 30000,
        }
    }
}

/// Result of a connection attempt
#[derive(Debug)]
pub enum ConnectionResult {
    Success(WebSocketStream<MaybeTlsStream<TcpStream>>),
    Timeout,
    Failed(String),
}

/// Establish WebSocket connection with retry logic
pub async fn connect_with_retry(
    url: &Url,
    config: &ConnectionConfig,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let start_time = Instant::now();
    let mut delay = config.initial_delay_ms;

    for attempt in 0..config.max_retries {
        if start_time.elapsed().as_millis() as u64 > config.max_timeout_ms {
            return Err(anyhow::anyhow!(
                "Connection timeout: Failed to connect within {}ms",
                config.max_timeout_ms
            ));
        }

        match connect_async(url.clone()).await {
            Ok((ws_stream, _)) => return Ok(ws_stream),
            Err(e) => {
                if attempt == config.max_retries - 1 {
                    return Err(anyhow::anyhow!(
                        "Failed to connect after {} attempts: {}",
                        config.max_retries,
                        e
                    ));
                }
                tokio::time::sleep(Duration::from_millis(delay)).await;
                delay = std::cmp::min(delay * 2, config.max_delay_ms);
            }
        }
    }

    Err(anyhow::anyhow!("Failed to connect to WebSocket: {}", url))
}

/// Parse and validate WebSocket URL
pub fn parse_websocket_url(url_str: &str) -> Result<Url> {
    let trimmed = url_str.trim();

    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("URL cannot be empty"));
    }

    let url = Url::parse(trimmed).with_context(|| format!("Invalid URL format: {}", trimmed))?;

    Ok(url)
}

/// Calculate delay for exponential backoff
pub fn calculate_backoff_delay(attempt: u32, config: &ConnectionConfig) -> Duration {
    let delay = config.initial_delay_ms * 2u64.pow(attempt.min(10));
    Duration::from_millis(std::cmp::min(delay, config.max_delay_ms))
}

/// Check if timeout has been exceeded
pub fn is_timeout_exceeded(start_time: Instant, max_timeout_ms: u64) -> bool {
    start_time.elapsed().as_millis() as u64 > max_timeout_ms
}
