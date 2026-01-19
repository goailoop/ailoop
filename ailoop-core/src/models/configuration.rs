//! Configuration data structures

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Logging level configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LogLevel {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "info")]
    #[default]
    Info,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "trace")]
    Trace,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    /// Default timeout in seconds for questions (0 = no timeout)
    pub timeout_seconds: Option<u32>,
    /// Default channel name
    pub default_channel: String,
    /// Logging verbosity level
    pub log_level: LogLevel,
    /// Server bind address
    pub server_host: String,
    /// Server port number
    pub server_port: u16,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(300), // 5 minutes default
            default_channel: "public".to_string(),
            log_level: LogLevel::Info,
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            max_connections: 100,
            max_message_size: 10240, // 10KB
        }
    }
}

impl Configuration {
    /// Load configuration from file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Configuration = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Return default configuration if file doesn't exist
            Ok(Configuration::default())
        }
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the XDG config directory path
    pub fn default_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir().ok_or("Could not determine config directory")?;
        Ok(config_dir.join("ailoop").join("config.toml"))
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate timeout
        if let Some(timeout) = self.timeout_seconds {
            if timeout > 3600 {
                errors.push("timeout_seconds cannot exceed 3600 (1 hour)".to_string());
            }
        }

        // Validate port (u16 is already 0-65535, so only check minimum)
        if self.server_port < 1024 {
            errors.push(
                "server_port must be at least 1024 (privileged ports not allowed)".to_string(),
            );
        }

        // Validate max connections
        if self.max_connections > 1000 {
            errors.push("max_connections cannot exceed 1000".to_string());
        }

        // Validate message size
        if self.max_message_size > 102400 {
            // 100KB
            errors.push("max_message_size cannot exceed 102400 bytes (100KB)".to_string());
        }

        // Validate channel name
        if !is_valid_channel_name(&self.default_channel) {
            errors.push("default_channel must match channel naming convention".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validate channel name according to naming convention
fn is_valid_channel_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && !first_char.is_ascii_digit() {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_alphabetic() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_configuration() {
        let config = Configuration::default();
        assert_eq!(config.timeout_seconds, Some(300));
        assert_eq!(config.default_channel, "public");
        assert_eq!(config.server_port, 8080);
    }

    #[test]
    fn test_configuration_validation() {
        let config = Configuration {
            timeout_seconds: Some(7200),                     // Invalid: too high
            default_channel: "invalid channel!".to_string(), // Invalid: spaces and special chars
            server_port: 80,                                 // Invalid: privileged port
            max_connections: 2000,                           // Invalid: too high
            max_message_size: 200000,                        // Invalid: too high
            ..Configuration::default()
        };

        let errors = config.validate().unwrap_err();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("timeout_seconds")));
        assert!(errors.iter().any(|e| e.contains("default_channel")));
        assert!(errors.iter().any(|e| e.contains("server_port")));
    }

    #[test]
    fn test_channel_name_validation() {
        assert!(is_valid_channel_name("valid-channel"));
        assert!(is_valid_channel_name("valid_channel"));
        assert!(is_valid_channel_name("channel123"));
        assert!(is_valid_channel_name("a"));

        assert!(!is_valid_channel_name(""));
        assert!(!is_valid_channel_name("-invalid-start"));
        assert!(!is_valid_channel_name("invalid space"));
        assert!(!is_valid_channel_name("invalid@symbol"));
        assert!(!is_valid_channel_name(&"a".repeat(65))); // Too long
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let config = Configuration {
            timeout_seconds: Some(120),
            default_channel: "test-channel".to_string(),
            ..Configuration::default()
        };

        // Save configuration
        config.save_to_file(&config_path).unwrap();
        assert!(config_path.exists());

        // Load configuration
        let loaded_config = Configuration::load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.timeout_seconds, Some(120));
        assert_eq!(loaded_config.default_channel, "test-channel");
    }
}
