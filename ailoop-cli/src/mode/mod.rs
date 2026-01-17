//! Mode detection component (COMP-001)
//!
//! Determines whether CLI commands should operate in direct mode or server mode
//! based on command-line arguments and environment variables.
//!
//! Implements interface IF-001: DetermineOperationMode
//! Manages entity ENTITY-008: OperationMode

mod detection;
mod operation_mode;

pub use detection::determine_operation_mode;
pub use operation_mode::{OperationMode, PrecedenceSource};

use thiserror::Error;

/// Error types for mode detection
#[derive(Debug, Error)]
pub enum ModeDetectionError {
    #[error("Invalid server URL format: {0}")]
    InvalidServerUrl(String),
}
