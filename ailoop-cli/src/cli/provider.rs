//! Provider subcommands (list, telegram test)

use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProviderCommands {
    /// List configured providers and their status (no secrets)
    List {
        /// Path to config file
        #[arg(long, default_value = "~/.config/ailoop/config.toml")]
        config: String,
    },
    /// Telegram provider
    Telegram {
        #[command(subcommand)]
        command: TelegramCommands,
    },
}

#[derive(Subcommand)]
pub enum TelegramCommands {
    /// Send a test message to the configured Telegram chat
    Test {
        /// Path to config file
        #[arg(long, default_value = "~/.config/ailoop/config.toml")]
        config: String,
    },
}
