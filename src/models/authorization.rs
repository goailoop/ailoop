//! Authorization data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authorization decision states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationDecision {
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "denied")]
    Denied,
    #[serde(rename = "timeout")]
    Timeout,
}

/// Authorization record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRecord {
    /// Unique authorization identifier
    pub id: Uuid,
    /// Channel context
    pub channel: String,
    /// Description of action requiring approval
    pub action: String,
    /// AI agent identifier that requested authorization
    pub requester: String,
    /// Final decision
    pub decision: AuthorizationDecision,
    /// Human user who made the decision (if applicable)
    pub human_user: Option<String>,
    /// When authorization was requested
    pub request_timestamp: DateTime<Utc>,
    /// When decision was made
    pub decision_timestamp: DateTime<Utc>,
    /// Additional context for the decision
    pub metadata: serde_json::Value,
}

impl AuthorizationRecord {
    /// Create a new authorization request
    pub fn new(channel: String, action: String, requester: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel,
            action,
            requester,
            decision: AuthorizationDecision::Timeout, // Will be updated
            human_user: None,
            request_timestamp: Utc::now(),
            decision_timestamp: Utc::now(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Record a decision
    pub fn record_decision(
        mut self,
        decision: AuthorizationDecision,
        human_user: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        self.decision = decision;
        self.human_user = human_user;
        self.decision_timestamp = Utc::now();
        if let Some(meta) = metadata {
            self.metadata = meta;
        }
        self
    }

    /// Check if authorization is approved
    pub fn is_approved(&self) -> bool {
        matches!(self.decision, AuthorizationDecision::Approved)
    }

    /// Check if authorization is denied
    pub fn is_denied(&self) -> bool {
        matches!(self.decision, AuthorizationDecision::Denied)
    }

    /// Check if authorization timed out
    pub fn is_timeout(&self) -> bool {
        matches!(self.decision, AuthorizationDecision::Timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_creation() {
        let auth = AuthorizationRecord::new(
            "admin".to_string(),
            "Deploy to production".to_string(),
            "agent-123".to_string(),
        );

        assert_eq!(auth.channel, "admin");
        assert_eq!(auth.action, "Deploy to production");
        assert_eq!(auth.requester, "agent-123");
        assert!(auth.is_timeout()); // Default state
        assert!(auth.human_user.is_none());
    }

    #[test]
    fn test_authorization_decision() {
        let auth = AuthorizationRecord::new(
            "admin".to_string(),
            "Delete database".to_string(),
            "agent-456".to_string(),
        );

        let decided_auth = auth.record_decision(
            AuthorizationDecision::Approved,
            Some("admin-user".to_string()),
            None,
        );

        assert!(decided_auth.is_approved());
        assert_eq!(decided_auth.human_user, Some("admin-user".to_string()));
    }
}