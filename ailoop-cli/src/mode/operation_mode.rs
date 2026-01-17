//! OperationMode entity (ENTITY-008)
//!
//! Represents the determined operation mode (direct or server) for a CLI command execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Operation mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Direct mode: operate locally in terminal
    Direct,
    /// Server mode: operate via WebSocket connection
    Server,
}

/// Source that determined the operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecedenceSource {
    /// AILOOP_SERVER environment variable was used
    AiloopServer,
    /// --server flag was used
    ServerFlag,
    /// Default (neither AILOOP_SERVER nor --server flag present)
    Default,
}

/// OperationMode entity (ENTITY-008)
///
/// Represents the determined operation mode with all required attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMode {
    /// Determined operation mode
    pub mode: Mode,
    /// Server URL to use (only present when mode is 'server')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    /// Which source was used to determine mode
    pub precedence_source: PrecedenceSource,
    /// Timestamp when mode was determined
    pub determined_at: DateTime<Utc>,
}

impl OperationMode {
    /// Create a new OperationMode instance
    pub fn new(
        mode: Mode,
        server_url: Option<String>,
        precedence_source: PrecedenceSource,
    ) -> Self {
        Self {
            mode,
            server_url,
            precedence_source,
            determined_at: Utc::now(),
        }
    }

    /// Create a direct mode instance
    pub fn direct(precedence_source: PrecedenceSource) -> Self {
        Self::new(Mode::Direct, None, precedence_source)
    }

    /// Create a server mode instance
    pub fn server(server_url: String, precedence_source: PrecedenceSource) -> Self {
        Self::new(Mode::Server, Some(server_url), precedence_source)
    }

    /// Check if mode is direct
    pub fn is_direct(&self) -> bool {
        self.mode == Mode::Direct
    }

    /// Check if mode is server
    pub fn is_server(&self) -> bool {
        self.mode == Mode::Server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_mode_direct() {
        let mode = OperationMode::direct(PrecedenceSource::Default);
        assert!(mode.is_direct());
        assert!(!mode.is_server());
        assert_eq!(mode.server_url, None);
        assert_eq!(mode.precedence_source, PrecedenceSource::Default);
    }

    #[test]
    fn test_operation_mode_server() {
        let url = "ws://localhost:8080".to_string();
        let mode = OperationMode::server(url.clone(), PrecedenceSource::AiloopServer);
        assert!(!mode.is_direct());
        assert!(mode.is_server());
        assert_eq!(mode.server_url, Some(url));
        assert_eq!(mode.precedence_source, PrecedenceSource::AiloopServer);
    }

    #[test]
    fn test_operation_mode_invariants() {
        // Direct mode must not have server_url
        let direct = OperationMode::direct(PrecedenceSource::Default);
        assert_eq!(direct.server_url, None);

        // Server mode must have server_url
        let server = OperationMode::server(
            "ws://localhost:8080".to_string(),
            PrecedenceSource::ServerFlag,
        );
        assert!(server.server_url.is_some());
    }
}
