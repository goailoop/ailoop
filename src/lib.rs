//! ailoop - Human-in-the-Loop CLI Tool
//!
//! This library provides the core functionality for ailoop, a tool that enables
//! AI agents to communicate with human users through structured interactions.

pub mod cli;
pub mod server;
pub mod channel;
pub mod models;
pub mod services;

// Re-export commonly used types
pub use models::*;
pub use services::*;