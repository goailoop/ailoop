//! Channel name validation

use thiserror::Error;

/// Errors that can occur during channel validation
#[derive(Error, Debug)]
pub enum ChannelValidationError {
    #[error("Channel name is empty")]
    Empty,

    #[error("Channel name is too long (max 64 characters)")]
    TooLong,

    #[error("Channel name must start with a letter or number")]
    InvalidStart,

    #[error("Channel name contains invalid characters (only letters, numbers, hyphens, and underscores allowed)")]
    InvalidCharacters,

    #[error("Channel name is reserved: {0}")]
    ReservedName(String),
}

/// Reserved channel names that cannot be used
const RESERVED_NAMES: &[&str] = &[
    "system",
    "admin",
    "internal",
    "reserved",
    "ailoop",
];

/// Validate a channel name according to the naming convention
pub fn validate_channel_name(name: &str) -> Result<(), ChannelValidationError> {
    // Check if empty
    if name.is_empty() {
        return Err(ChannelValidationError::Empty);
    }

    // Check length
    if name.len() > 64 {
        return Err(ChannelValidationError::TooLong);
    }

    // Check first character
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && !first_char.is_ascii_digit() {
        return Err(ChannelValidationError::InvalidStart);
    }

    // Check for reserved names
    if RESERVED_NAMES.contains(&name.to_lowercase().as_str()) {
        return Err(ChannelValidationError::ReservedName(name.to_string()));
    }

    // Check all characters are valid
    for ch in name.chars() {
        if !ch.is_ascii_alphabetic() && !ch.is_ascii_digit() && ch != '-' && ch != '_' {
            return Err(ChannelValidationError::InvalidCharacters);
        }
    }

    Ok(())
}

/// Check if a channel name is valid (convenience function)
pub fn is_valid_channel_name(name: &str) -> bool {
    validate_channel_name(name).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_channel_names() {
        assert!(is_valid_channel_name("valid-channel"));
        assert!(is_valid_channel_name("valid_channel"));
        assert!(is_valid_channel_name("channel123"));
        assert!(is_valid_channel_name("a"));
        assert!(is_valid_channel_name("Channel-123_Test"));
    }

    #[test]
    fn test_invalid_channel_names() {
        assert!(!is_valid_channel_name(""));
        assert!(!is_valid_channel_name("-invalid-start"));
        assert!(!is_valid_channel_name("_invalid-start"));
        assert!(!is_valid_channel_name("invalid space"));
        assert!(!is_valid_channel_name("invalid@symbol"));
        assert!(!is_valid_channel_name("system"));
        assert!(!is_valid_channel_name("admin"));

        // Test length limit
        let long_name = "a".repeat(65);
        assert!(!is_valid_channel_name(&long_name));
    }

    #[test]
    fn test_validation_error_messages() {
        match validate_channel_name("") {
            Err(ChannelValidationError::Empty) => (),
            _ => panic!("Expected Empty error"),
        }

        match validate_channel_name("-invalid") {
            Err(ChannelValidationError::InvalidStart) => (),
            _ => panic!("Expected InvalidStart error"),
        }

        match validate_channel_name("system") {
            Err(ChannelValidationError::ReservedName(name)) => assert_eq!(name, "system"),
            _ => panic!("Expected ReservedName error"),
        }

        match validate_channel_name("invalid@name") {
            Err(ChannelValidationError::InvalidCharacters) => (),
            _ => panic!("Expected InvalidCharacters error"),
        }
    }
}