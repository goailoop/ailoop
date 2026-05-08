//! Shared helpers for ailoop-core integration tests.

use anyhow::{Context, Result};

/// Reserves a single free ephemeral TCP port on `host`.
pub fn find_free_port(host: &str) -> Result<u16> {
    let listener =
        std::net::TcpListener::bind((host, 0)).context("Failed to bind ephemeral port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
