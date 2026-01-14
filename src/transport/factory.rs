//! Transport factory for creating transport instances

use crate::transport::{file::FileTransport, websocket::WebSocketTransport, Transport};
use anyhow::{Context, Result};

/// Transport type identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportType {
    WebSocket,
    File,
}

/// Configuration for creating a transport
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub transport_type: TransportType,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub channel: String,
    pub client_id: Option<String>,
}

/// Create a transport instance based on configuration
pub fn create_transport(config: TransportConfig) -> Result<Box<dyn Transport>> {
    match config.transport_type {
        TransportType::WebSocket => {
            let url = config.url.context("WebSocket transport requires URL")?;
            Ok(Box::new(WebSocketTransport::new(
                url,
                config.channel,
                config.client_id,
            )?))
        }
        TransportType::File => {
            let file_path = config
                .file_path
                .context("File transport requires file path")?;
            Ok(Box::new(FileTransport::new(file_path, config.channel)?))
        }
    }
}
