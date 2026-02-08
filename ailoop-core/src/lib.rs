//! # Ailoop Core Library
//!
//! Shared core functionality for ailoop including models, transport/client helpers,
//! server/channel management, and an extensible agent event parser system (cursor, jsonl, opencode).
//!
//! ## Parser Module
//!
//! The `parser` module provides an extensible parser system for converting agent output
//! into standardized `AgentEvent` structures. Key types include:
//! - `AgentEvent`: Unified event structure with type, content, and metadata
//! - `EventType`: Classification of events (System, User, Assistant, ToolCall, etc.)
//! - `InputFormat`: Supported input formats (Json, StreamJson, Text)
//! - `AgentParser`: Trait for parser implementations
//! - `create_parser`: Factory function to create parser instances
//!
//! Supported agent types:
//! - `cursor`: Cursor CLI output parser
//! - `jsonl`: Generic JSONL parser with agent_type tags
//! - `opencode`: OpenCode stream JSON parser

pub mod channel;
pub mod client;
pub mod models;
pub mod parser;
pub mod server;
pub mod services;
pub mod transport;
pub mod workflow;
