/// Top-level error type for the ailoop-server crate.
#[derive(Debug, thiserror::Error)]
pub enum AiloopError {
    #[error("Invalid base_path: {0}")]
    InvalidBasePath(String),

    #[error("Server is shutting down")]
    ServerShuttingDown,

    #[error("Authentication required")]
    Unauthorized,

    #[error("Address parse error: {0}")]
    AddressParse(#[from] std::net::AddrParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}
