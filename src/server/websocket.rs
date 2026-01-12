//! WebSocket server implementation

use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::stream::StreamExt;
use std::net::SocketAddr;

/// WebSocket server for handling ailoop connections
pub struct WebSocketServer {
    address: SocketAddr,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(host: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let address = format!("{}:{}", host, port).parse()?;
        Ok(Self { address })
    }

    /// Start the WebSocket server
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.address).await?;
        println!("WebSocket server listening on {}", self.address);

        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(async move {
                println!("New connection from: {}", addr);

                match accept_async(stream).await {
                    Ok(ws_stream) => {
                        println!("WebSocket connection established with {}", addr);

                        let (_write, mut read) = ws_stream.split();

                        // For now, just echo back messages
                        while let Some(message) = read.next().await {
                            match message {
                                Ok(msg) => {
                                    println!("Received message: {:?}", msg);
                                }
                                Err(e) => {
                                    println!("Error receiving message: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("WebSocket handshake failed: {}", e);
                    }
                }
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_server_creation() {
        let server = WebSocketServer::new("127.0.0.1", 8080);
        assert!(server.is_ok());
    }

    #[test]
    fn test_websocket_server_address() {
        let server = WebSocketServer::new("127.0.0.1", 8080).unwrap();
        assert_eq!(server.address.to_string(), "127.0.0.1:8080");
    }
}