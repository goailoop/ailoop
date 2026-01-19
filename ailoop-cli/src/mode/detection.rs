//! Mode detection implementation (IF-001: DetermineOperationMode)
//!
//! Determines operation mode based on command-line arguments and environment variables.
//! Implements REQ-001, REQ-002, REQ-003.

use super::{ModeDetectionError, OperationMode, PrecedenceSource};
use std::env;
use std::time::Instant;

/// Determine operation mode based on command-line arguments and environment variables
///
/// Implements interface IF-001: DetermineOperationMode
///
/// # Arguments
/// * `server_flag` - Optional server URL from --server flag
///
/// # Returns
/// * `Ok(OperationMode)` - Determined operation mode
/// * `Err(ModeDetectionError::InvalidServerUrl)` - If server URL format is invalid
///
/// # Behavior
/// 1. AILOOP_SERVER environment variable takes precedence over --server flag (REQ-003, REQ-004)
/// 2. If AILOOP_SERVER is set, use server mode with that URL
/// 3. If --server flag is provided (and AILOOP_SERVER is not set), use server mode with flag URL
/// 4. If neither is present, use direct mode (REQ-002)
/// 5. Must complete within 100ms (QC-001)
pub fn determine_operation_mode(
    server_flag: Option<String>,
) -> Result<OperationMode, ModeDetectionError> {
    determine_operation_mode_with_env(server_flag, None)
}

pub fn determine_operation_mode_with_env(
    server_flag: Option<String>,
    env_override: Option<Option<String>>,
) -> Result<OperationMode, ModeDetectionError> {
    let start = Instant::now();

    // Check AILOOP_SERVER environment variable first (takes precedence)
    // Allow override for testing to avoid race conditions
    let env_server = env_override.unwrap_or_else(|| env::var("AILOOP_SERVER").ok());

    let result = if let Some(env_url) = env_server {
        // AILOOP_SERVER takes precedence (REQ-003, REQ-004)
        let server_url = convert_to_websocket_url(&env_url)?;
        OperationMode::server(server_url, PrecedenceSource::AiloopServer)
    } else if let Some(flag_url) = server_flag {
        // --server flag is used when AILOOP_SERVER is not set
        let server_url = convert_to_websocket_url(&flag_url)?;
        OperationMode::server(server_url, PrecedenceSource::ServerFlag)
    } else {
        // Default to direct mode (REQ-002)
        OperationMode::direct(PrecedenceSource::Default)
    };

    // Verify completion within 100ms (QC-001)
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 100 {
        // Log warning but don't fail - this is a quality contract, not a hard requirement
        eprintln!(
            "Warning: Mode detection took {}ms, exceeding 100ms threshold",
            elapsed.as_millis()
        );
    }

    Ok(result)
}

/// Convert HTTP/HTTPS URL to WebSocket URL
///
/// Converts http:// to ws:// and https:// to wss://
/// If URL already starts with ws:// or wss://, returns it as-is.
/// If URL is invalid, returns an error.
fn convert_to_websocket_url(url: &str) -> Result<String, ModeDetectionError> {
    let trimmed = url.trim();

    // Validate URL format (basic validation)
    if trimmed.is_empty() {
        return Err(ModeDetectionError::InvalidServerUrl(
            "URL cannot be empty".to_string(),
        ));
    }

    // Check if already a WebSocket URL
    if trimmed.starts_with("ws://") || trimmed.starts_with("wss://") {
        // Validate the URL structure
        if url::Url::parse(trimmed).is_err() {
            return Err(ModeDetectionError::InvalidServerUrl(format!(
                "Invalid WebSocket URL format: {}",
                trimmed
            )));
        }
        return Ok(trimmed.to_string());
    }

    // Convert HTTP/HTTPS to WebSocket
    if trimmed.starts_with("http://") {
        let ws_url = trimmed.replacen("http://", "ws://", 1);
        // Validate the converted URL
        if url::Url::parse(&ws_url).is_err() {
            return Err(ModeDetectionError::InvalidServerUrl(format!(
                "Invalid URL format after conversion: {}",
                ws_url
            )));
        }
        return Ok(ws_url);
    }

    if trimmed.starts_with("https://") {
        let wss_url = trimmed.replacen("https://", "wss://", 1);
        // Validate the converted URL
        if url::Url::parse(&wss_url).is_err() {
            return Err(ModeDetectionError::InvalidServerUrl(format!(
                "Invalid URL format after conversion: {}",
                wss_url
            )));
        }
        return Ok(wss_url);
    }

    // If URL doesn't start with http://, https://, ws://, or wss://, treat as invalid
    Err(ModeDetectionError::InvalidServerUrl(format!(
        "URL must start with http://, https://, ws://, or wss://: {}",
        trimmed
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn clear_env() {
        env::remove_var("AILOOP_SERVER");
        env::remove_var("AILOOP_MODE");
    }

    /// TC-REQ-001-01: Verify direct mode detection when AILOOP_SERVER is absent
    ///
    /// Given: AILOOP_SERVER environment variable is not set
    /// When: Execute 'ailoop ask "test"' command
    /// Then:
    ///   - System determines operation_mode = 'direct'
    ///   - Mode detection completes within 100ms
    ///   - Question is displayed in local terminal (not WebSocket connection attempted)
    #[test]
    fn test_tc_req_001_01_direct_mode_when_env_not_set() {
        // Given: AILOOP_SERVER environment variable is not set
        clear_env();

        // When: Execute mode detection (simulating 'ailoop ask "test"' command)
        let start = Instant::now();
        let mode = determine_operation_mode(None).unwrap();
        let elapsed = start.elapsed();

        // Then: System determines operation_mode = 'direct'
        assert!(
            mode.is_direct(),
            "Mode should be direct when no env var or flag is set"
        );
        assert_eq!(mode.precedence_source, PrecedenceSource::Default);
        assert_eq!(mode.server_url, None);

        // Then: Mode detection completes within 100ms
        assert!(
            elapsed.as_millis() < 100,
            "Mode detection took {}ms, exceeding 100ms threshold",
            elapsed.as_millis()
        );

        // Clean up after test
        clear_env();
    }

    #[test]
    fn test_server_mode_from_flag() {
        // Test with explicit environment (None = not set)
        let mode = determine_operation_mode_with_env(
            Some("http://localhost:8080".to_string()),
            Some(None),
        )
        .unwrap();
        assert!(
            mode.is_server(),
            "Mode should be server when --server flag is provided"
        );
        assert_eq!(mode.precedence_source, PrecedenceSource::ServerFlag);
        assert!(mode.server_url.is_some());
        assert!(mode.server_url.unwrap().starts_with("ws://"));
        // Clean up after test
        clear_env();
    }

    /// TC-REQ-001-02: Verify server mode detection when AILOOP_SERVER is present
    ///
    /// Given: AILOOP_SERVER=http://localhost:8080 environment variable is set
    /// When: Execute 'ailoop ask "test"' command
    /// Then:
    ///   - System determines operation_mode = 'server'
    ///   - Mode detection completes within 100ms
    ///   - System attempts WebSocket connection (no local terminal prompt)
    #[test]
    fn test_tc_req_001_02_server_mode_when_env_set() {
        // Given: AILOOP_SERVER=http://localhost:8080 environment variable is set
        // When: Execute mode detection (simulating 'ailoop ask "test"' command)
        let start = Instant::now();
        let mode = determine_operation_mode_with_env(
            None,
            Some(Some("http://localhost:8080".to_string())),
        )
        .unwrap();
        let elapsed = start.elapsed();

        // Then: System determines operation_mode = 'server'
        assert!(
            mode.is_server(),
            "Mode should be server when AILOOP_SERVER env var is set"
        );
        assert_eq!(mode.precedence_source, PrecedenceSource::AiloopServer);

        // Then: System attempts WebSocket connection (URL converted correctly)
        assert!(
            mode.server_url.is_some(),
            "Server URL should be present in server mode"
        );
        let server_url = mode.server_url.unwrap();
        assert_eq!(
            server_url, "ws://localhost:8080",
            "URL should be converted from http:// to ws://"
        );

        // Then: Mode detection completes within 100ms
        assert!(
            elapsed.as_millis() < 100,
            "Mode detection took {}ms, exceeding 100ms threshold",
            elapsed.as_millis()
        );

        // Clean up after test
        clear_env();
    }

    /// TC-REQ-003-01: Verify server mode activation with AILOOP_SERVER
    ///
    /// Given: AILOOP_SERVER=http://localhost:8080 environment variable is set
    /// When: Execute 'ailoop ask "test question"' command
    /// Then:
    ///   - System attempts WebSocket connection to ws://localhost:8080
    ///   - No local terminal prompt is displayed (mode is server, not direct)
    ///   - Message is sent via WebSocket connection (mode detection enables this)
    ///
    /// Note: This test verifies mode detection component behavior. Actual WebSocket connection
    /// and message sending are handled by other components (COMP-003, COMP-004).
    #[test]
    fn test_tc_req_003_01_server_mode_activation_with_ailoop_server() {
        // Given: AILOOP_SERVER=http://localhost:8080 environment variable is set
        // When: Execute mode detection (simulating 'ailoop ask "test question"' command)
        let mode = determine_operation_mode_with_env(
            None,
            Some(Some("http://localhost:8080".to_string())),
        )
        .unwrap();

        // Then: System attempts WebSocket connection to ws://localhost:8080
        assert!(
            mode.is_server(),
            "Mode should be server when AILOOP_SERVER is set"
        );
        assert_eq!(mode.precedence_source, PrecedenceSource::AiloopServer);
        let server_url = mode
            .server_url
            .as_ref()
            .expect("Server URL must be present in server mode");
        assert_eq!(
            server_url, "ws://localhost:8080",
            "WebSocket URL should be ws://localhost:8080"
        );

        // Then: No local terminal prompt is displayed (mode is server, not direct)
        assert!(
            !mode.is_direct(),
            "Mode should not be direct when AILOOP_SERVER is set"
        );

        // Then: Message is sent via WebSocket connection (mode detection enables this)
        // Note: Actual message sending is tested in integration tests. This test verifies
        // that mode detection correctly enables server mode, which allows WebSocket communication.
        assert!(
            mode.is_server(),
            "Server mode must be enabled for WebSocket message sending"
        );

        // No cleanup needed - test uses isolated environment
    }

    #[test]
    fn test_env_takes_precedence_over_flag() {
        // Test that AILOOP_SERVER takes precedence over --server flag
        let mode = determine_operation_mode_with_env(
            Some("http://flag-server:8080".to_string()),
            Some(Some("http://env-server:8080".to_string())),
        )
        .unwrap();

        // No cleanup needed - test uses isolated environment

        assert!(
            mode.is_server(),
            "Mode should be server when AILOOP_SERVER env var is set"
        );
        assert_eq!(
            mode.precedence_source,
            PrecedenceSource::AiloopServer,
            "AILOOP_SERVER should take precedence over flag"
        );
        let server_url = mode.server_url.unwrap();
        assert!(
            server_url.contains("env-server"),
            "Server URL should come from env var, not flag. Got: {}",
            server_url
        );
    }

    #[test]
    fn test_url_conversion_http_to_ws() {
        let result = convert_to_websocket_url("http://localhost:8080").unwrap();
        assert_eq!(result, "ws://localhost:8080");
    }

    #[test]
    fn test_url_conversion_https_to_wss() {
        let result = convert_to_websocket_url("https://example.com:443").unwrap();
        assert_eq!(result, "wss://example.com:443");
    }

    #[test]
    fn test_url_already_websocket() {
        let result = convert_to_websocket_url("ws://localhost:8080").unwrap();
        assert_eq!(result, "ws://localhost:8080");

        let result = convert_to_websocket_url("wss://example.com:443").unwrap();
        assert_eq!(result, "wss://example.com:443");
    }

    #[test]
    fn test_invalid_url_empty() {
        let result = convert_to_websocket_url("");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeDetectionError::InvalidServerUrl(_))
        ));
    }

    #[test]
    fn test_invalid_url_format() {
        let result = convert_to_websocket_url("not-a-url");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeDetectionError::InvalidServerUrl(_))
        ));
    }

    #[test]
    fn test_performance_requirement() {
        clear_env();
        let start = Instant::now();
        let _mode = determine_operation_mode(None).unwrap();
        let elapsed = start.elapsed();
        // Should complete well within 100ms
        assert!(
            elapsed.as_millis() < 100,
            "Mode detection took {}ms",
            elapsed.as_millis()
        );
        // Clean up after test
        clear_env();
    }
}
