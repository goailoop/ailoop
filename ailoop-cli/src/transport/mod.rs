//! Transport abstraction for message delivery
//!
//! This module provides a trait-based transport system that allows messages
//! to be sent through various mechanisms (WebSocket, file, Kafka, Redis, etc.)
//! without the message converter needing to know implementation details.

use crate::models::Message;
use anyhow::Result;
use async_trait::async_trait;

/// Abstract transport interface for message delivery
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a message through the transport
    async fn send(&mut self, message: Message) -> Result<()>;

    /// Flush any buffered messages
    async fn flush(&mut self) -> Result<()>;

    /// Close the transport connection
    async fn close(&mut self) -> Result<()>;

    /// Get transport name for logging
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

pub mod factory;
pub mod file;
pub mod websocket;
