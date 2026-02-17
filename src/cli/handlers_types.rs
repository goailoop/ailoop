//! Common types for CLI command handlers

use crate::models::ResponseType;

/// Parameters for ask/authorize command handlers
#[derive(Debug, Clone)]
pub struct CommandParams {
    pub channel: String,
    pub timeout_secs: u32,
    pub server: String,
    pub json: bool,
}

impl CommandParams {
    pub fn new(channel: String, timeout_secs: u32, server: String, json: bool) -> Self {
        Self {
            channel,
            timeout_secs,
            server,
            json,
        }
    }
}

/// Result of collecting user input
#[derive(Debug, Clone)]
pub enum UserInputResult {
    Answer(String),
    Timeout,
    Cancelled,
}

/// Authorization decision
#[derive(Debug, Clone, PartialEq)]
pub enum AuthorizationDecision {
    Approved,
    Denied,
}

/// Response handling result
#[derive(Debug)]
pub enum ResponseHandlingResult {
    Success,
    Timeout,
    Cancelled,
    Unknown(ResponseType),
}

/// JSON response builder for consistent error/response formatting
pub struct JsonResponseBuilder {
    channel: String,
}

impl JsonResponseBuilder {
    pub fn new(channel: String) -> Self {
        Self { channel }
    }

    pub fn error(&self, error_type: &str, message: &str) -> String {
        serde_json::json!({
            "error": error_type,
            "message": message,
            "channel": self.channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string()
    }

    pub fn response(&self, response_text: &str) -> String {
        serde_json::json!({
            "response": response_text,
            "channel": self.channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string()
    }

    pub fn authorization(&self, authorized: bool, action: &str) -> String {
        serde_json::json!({
            "authorized": authorized,
            "action": action,
            "channel": self.channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string()
    }

    pub fn response_with_metadata(
        &self,
        response_text: &str,
        metadata: Option<&serde_json::Value>,
    ) -> String {
        let mut response = serde_json::json!({
            "response": response_text,
            "channel": self.channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        if let Some(meta) = metadata {
            response["metadata"] = meta.clone();
        }

        serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string())
    }
}

/// Format and print JSON or plain output
pub fn print_output(json: bool, json_builder: &JsonResponseBuilder, plain_msg: &str) {
    if json {
        println!("{}", json_builder.response(plain_msg));
    } else {
        println!("{}", plain_msg);
    }
}

/// Print error output
pub fn print_error_output(
    json: bool,
    json_builder: &JsonResponseBuilder,
    error_type: &str,
    plain_msg: &str,
) {
    if json {
        println!("{}", json_builder.error(error_type, plain_msg));
    } else {
        println!("{}", plain_msg);
    }
}
