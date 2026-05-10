pub mod middleware;
pub mod server;

mod config;
mod error;
mod state;

// Composable library API
pub use crate::config::{AuthConfig, CorsConfig, ServeConfig};
pub use crate::error::AiloopError;
pub use crate::server::core::{router, spawn_background_tasks};
pub use crate::state::AiloopAppState;

// Convenience wrapper (kept for backward compatibility)
pub use server::AiloopServer;
pub use server::ServerStatus;
