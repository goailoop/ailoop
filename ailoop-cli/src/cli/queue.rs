//! CLI arguments for the `ailoop queue` subcommand.

use clap::Args;

#[derive(Args)]
pub struct QueueArgs {
    /// Server base URL (default: AILOOP_SERVER env or http://127.0.0.1:8080)
    #[arg(long, default_value = "")]
    pub server: String,

    /// Filter by channel name
    #[arg(long)]
    pub channel: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}
