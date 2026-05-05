//! Shared helpers for ailoop-core integration tests.

use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;

/// Serialize port reservation across parallel `cargo test` processes (separate test binaries).
pub fn port_allocation_lock() -> io::Result<std::fs::File> {
    let path: PathBuf = std::env::temp_dir().join("ailoop-workspace-integration-server-port.lock");
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    f.lock_exclusive()?;
    Ok(f)
}

/// Reserves a single free ephemeral TCP port on `host`.
/// MUST be called while holding `port_allocation_lock()`.
pub fn find_free_port(host: &str) -> Result<u16> {
    let listener =
        std::net::TcpListener::bind((host, 0)).context("Failed to bind ephemeral port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
