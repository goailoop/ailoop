//! ailoop - Human-in-the-Loop CLI Tool
//!
//! This library provides the core functionality for ailoop, a tool that enables
//! AI agents to communicate with human users through structured interactions.

pub mod channel;
pub mod cli;
pub mod mode;
pub mod models;
pub mod server;
pub mod services;
pub mod transport;

// Re-export commonly used types
pub use models::*;
pub use services::*;
