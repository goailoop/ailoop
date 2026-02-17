//! Common types for server handlers

use crate::models::{Message, ResponseType};

/// Result of user input collection
#[derive(Debug, Clone)]
pub enum InputResult {
    Answer(String),
    Timeout,
    Cancelled,
    Skip,
}

/// Result of authorization decision
#[derive(Debug, Clone, PartialEq)]
pub enum AuthDecision {
    Approved,
    Denied,
    Skip,
}

/// Question handling context
#[derive(Debug, Clone)]
pub struct QuestionContext {
    pub question_text: String,
    pub timeout_secs: u32,
    pub choices: Option<Vec<String>>,
}

impl QuestionContext {
    pub fn new(question_text: String, timeout_secs: u32, choices: Option<Vec<String>>) -> Self {
        Self {
            question_text,
            timeout_secs,
            choices,
        }
    }
}

/// Authorization handling context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub action: String,
    pub timeout_secs: u32,
}

impl AuthContext {
    pub fn new(action: String, timeout_secs: u32) -> Self {
        Self {
            action,
            timeout_secs,
        }
    }
}

/// Message dispatch result
#[derive(Debug, Clone)]
pub enum DispatchResult {
    Response(ResponseType),
    NoAction,
}

/// Helper to process multiple choice answers
pub fn process_multiple_choice(
    input: &str,
    choices: &Option<Vec<String>>,
) -> (String, Option<usize>) {
    let trimmed = input.trim();

    if let Some(choices_list) = choices {
        if let Ok(num) = trimmed.parse::<usize>() {
            if num >= 1 && num <= choices_list.len() {
                let index = num - 1;
                let selected = choices_list[index].clone();
                return (selected, Some(index));
            }
        }
        for (idx, choice) in choices_list.iter().enumerate() {
            if choice.trim().eq_ignore_ascii_case(trimmed) {
                return (choice.clone(), Some(idx));
            }
        }
    }

    (trimmed.to_string(), None)
}

/// Parse authorization input
pub fn parse_authorization_input(input: &str) -> AuthDecision {
    let normalized = input.trim().to_lowercase();
    match normalized.as_str() {
        "y" | "yes" | "authorized" | "approve" | "ok" | "" => AuthDecision::Approved,
        "n" | "no" | "denied" | "deny" | "reject" => AuthDecision::Denied,
        _ => AuthDecision::Approved,
    }
}

/// Create question response metadata
pub fn create_response_metadata(
    selected_index: Option<usize>,
    choices: &Option<Vec<String>>,
) -> Option<serde_json::Value> {
    if let Some(idx) = selected_index {
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "index".to_string(),
            serde_json::Value::Number(serde_json::Number::from(idx)),
        );
        if let Some(choices_list) = choices {
            if let Some(selected_choice) = choices_list.get(idx) {
                metadata.insert(
                    "value".to_string(),
                    serde_json::Value::String(selected_choice.clone()),
                );
            }
        }
        Some(serde_json::Value::Object(metadata))
    } else {
        None
    }
}
