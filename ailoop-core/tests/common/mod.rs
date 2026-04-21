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

/// Pick adjacent `(ws_port, ws_port + 1)` with both ports free at reservation time.
pub fn find_free_adjacent_port_pair(host: &str) -> Result<(u16, u16)> {
    for attempt in 0..100 {
        let ws_listener = std::net::TcpListener::bind((host, 0)).with_context(|| {
            format!(
                "Failed to bind ephemeral port on {} (attempt {})",
                host,
                attempt + 1
            )
        })?;
        let ws_port = ws_listener
            .local_addr()
            .context("Failed to get local addr for ws listener")?
            .port();

        if ws_port == u16::MAX {
            drop(ws_listener);
            continue;
        }

        let http_port = ws_port + 1;

        match std::net::TcpListener::bind((host, http_port)) {
            Ok(http_listener) => {
                let ports = (ws_port, http_port);
                drop(http_listener);
                drop(ws_listener);
                return Ok(ports);
            }
            Err(e) => {
                drop(ws_listener);
                if attempt < 99 {
                    let delay_ms = std::cmp::min(100, 10 * (1 << std::cmp::min(attempt / 10, 3)));
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                }
                if (attempt + 1) % 20 == 0 {
                    eprintln!(
                        "Warning: find_free_adjacent_port_pair attempt {} failed: HTTP port unavailable (last error: {})",
                        attempt + 1,
                        e
                    );
                }
                continue;
            }
        }
    }
    Err(anyhow::anyhow!(
        "Failed to find a free adjacent port pair after 100 attempts. \
         This suggests severe port exhaustion or a system issue."
    ))
}
