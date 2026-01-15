//! Logging service

use crate::models::LogLevel;

/// Initialize logging with the specified level
pub fn init_logging(level: LogLevel) -> Result<(), Box<dyn std::error::Error>> {
    let filter = match level {
        LogLevel::Error => "ailoop=error",
        LogLevel::Warn => "ailoop=warn",
        LogLevel::Info => "ailoop=info",
        LogLevel::Debug => "ailoop=debug",
        LogLevel::Trace => "ailoop=trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    Ok(())
}

/// Log an interaction event
pub fn log_interaction(event_type: &str, channel: &str, details: Option<&str>) {
    tracing::info!(
        event_type = event_type,
        channel = channel,
        details = details.unwrap_or(""),
        "Interaction logged"
    );
}

/// Log a security event (always logged regardless of level)
pub fn log_security_event(event_type: &str, channel: &str, user: Option<&str>, details: &str) {
    tracing::warn!(
        event_type = event_type,
        channel = channel,
        user = user.unwrap_or("unknown"),
        details = details,
        "Security event"
    );
}

/// Log a system error
pub fn log_error(error: &str, context: Option<&str>) {
    tracing::error!(
        error = error,
        context = context.unwrap_or(""),
        "System error occurred"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_logging() {
        INIT.call_once(|| {
            let _ = init_logging(LogLevel::Info);
        });
    }

    #[test]
    fn test_logging_initialization() {
        // Just test that initialization doesn't panic
        let _ = init_logging(LogLevel::Info);
    }

    #[test]
    fn test_log_functions() {
        init_test_logging();

        // These should not panic
        log_interaction("test", "test-channel", Some("test details"));
        log_security_event("test", "test-channel", Some("test-user"), "test details");
        log_error("test error", Some("test context"));
    }
}
