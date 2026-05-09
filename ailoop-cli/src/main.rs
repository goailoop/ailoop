mod cli;
mod mode;
mod parser;

// Re-export from ailoop-core
// use ailoop_core::*; // Not used in main.rs currently

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::handlers;

#[derive(clap::ValueEnum, Clone, Debug, Default)]
enum AuthorizeDefault {
    #[default]
    Yes,
    No,
}

#[derive(Parser)]
#[command(name = "ailoop")]
#[command(version)]
#[command(about = "Human-in-the-Loop CLI Tool for AI Agent Communication")]
#[command(
    help_template = "{name} - {version}\n{about}\n\n{usage-heading}\n  {usage}\n\n{all-args}{options}\n"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a structured decision and collect human selection.
    ///
    /// Provide a JSON-encoded decision via --decision-json. The JSON must contain:
    ///   decision_id, summary, options (array with id+label), and optionally
    ///   context_markdown, recommendation, timeout_seconds.
    ///
    /// Example:
    ///   ailoop ask --decision-json '{
    ///     "decision_id": "deploy",
    ///     "summary": "Which deployment strategy?",
    ///     "options": [
    ///       {"id": "blue-green", "label": "Blue/Green"},
    ///       {"id": "canary", "label": "Canary"}
    ///     ],
    ///     "timeout_seconds": 300
    ///   }'
    ///
    /// JSON Response Format (with --json):
    ///   {
    ///     "response": "blue-green",
    ///     "channel": "public",
    ///     "timestamp": "...",
    ///     "metadata": {
    ///       "option_id": "blue-green",
    ///       "label": "Blue/Green",
    ///       "index": 0
    ///     }
    ///   }
    Ask {
        /// JSON-encoded decision payload (decision_id, summary, options, etc.)
        #[arg(long = "decision-json")]
        decision_json: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Response timeout in seconds (0 = no timeout, overrides decision timeout_seconds)
        #[arg(short, long, default_value = "0")]
        timeout: u32,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format. Includes option_id, label, and index in metadata.
        #[arg(long)]
        json: bool,
    },

    /// Request authorization for a critical action
    Authorize {
        /// Description of action requiring authorization
        action: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Authorization timeout in seconds (default: 300)
        #[arg(short, long, default_value = "300")]
        timeout: u32,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Default decision when ENTER is pressed (yes or no)
        #[arg(long = "default", default_value = "yes")]
        default: AuthorizeDefault,
    },

    /// Send a notification message
    Say {
        /// Notification message text
        message: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Message priority level
        #[arg(short, long, default_value = "normal")]
        priority: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,
    },

    /// Start ailoop server for multi-agent communication
    Serve {
        /// Server bind address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port number
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Default channel name
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Enable the embedded web UI on the HTTP API port (port+1)
        #[arg(long)]
        web: bool,
    },

    /// Configure ailoop settings interactively
    Config {
        /// Start interactive configuration setup
        #[arg(long)]
        init: bool,

        /// Path to configuration file
        #[arg(long, default_value = "~/.config/ailoop/config.toml")]
        config_file: String,
    },

    /// Display an image to the user
    Image {
        /// Image file path or URL
        image_path: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,
    },

    /// Suggest user to navigate to a URL
    Navigate {
        /// URL to navigate to
        url: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,
    },

    /// Forward agent output to ailoop server
    Forward {
        /// Channel name for messages
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Agent type (cursor, jsonl, opencode, or auto-detect)
        #[arg(long)]
        agent_type: Option<String>,

        /// Input format (json, stream-json, text)
        #[arg(long, default_value = "stream-json")]
        format: String,

        /// Transport type (websocket, file)
        #[arg(long, default_value = "websocket")]
        transport: String,

        /// WebSocket server URL (for websocket transport)
        #[arg(long, default_value = "ws://127.0.0.1:8080")]
        url: Option<String>,

        /// Output file path (for file transport)
        #[arg(long)]
        output: Option<String>,

        /// Client ID for tracking
        #[arg(long)]
        client_id: Option<String>,

        /// Input file path (if not reading from stdin)
        #[arg(long)]
        input: Option<String>,
    },

    /// Task management commands
    Task {
        #[command(subcommand)]
        command: cli::task::TaskCommands,
    },

    /// Provider status and test (e.g. Telegram)
    Provider {
        #[command(subcommand)]
        command: cli::provider::ProviderCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ask {
            decision_json,
            channel,
            timeout,
            server,
            json,
        } => {
            handlers::handle_ask(decision_json, channel, timeout, server, json).await?;
        }
        Commands::Authorize {
            action,
            channel,
            timeout,
            server,
            json,
            default,
        } => {
            let default_yes = matches!(default, AuthorizeDefault::Yes);
            handlers::handle_authorize(action, channel, timeout, server, json, default_yes).await?;
        }
        Commands::Say {
            message,
            channel,
            priority,
            server,
        } => {
            handlers::handle_say(message, channel, priority, server).await?;
        }
        Commands::Serve {
            host,
            port,
            channel,
            web,
        } => {
            handlers::handle_serve(host, port, channel, web).await?;
        }
        Commands::Config { init, config_file } => {
            if init {
                handlers::handle_config_init(config_file).await?;
            } else {
                println!("Config command requires --init flag");
                println!("Usage: ailoop config --init [--config-file PATH]");
            }
        }
        Commands::Image {
            image_path,
            channel,
            server,
        } => {
            handlers::handle_image(image_path, channel, server).await?;
        }
        Commands::Navigate {
            url,
            channel,
            server,
        } => {
            handlers::handle_navigate(url, channel, server).await?;
        }
        Commands::Forward {
            channel,
            agent_type,
            format,
            transport,
            url,
            output,
            client_id,
            input,
        } => {
            handlers::handle_forward(
                channel, agent_type, format, transport, url, output, client_id, input,
            )
            .await?;
        }
        Commands::Task { command } => {
            crate::cli::task_handlers::handle_task_commands(command).await?;
        }
        Commands::Provider { command } => {
            crate::cli::provider_handlers::handle_provider_commands(command).await?;
        }
    }

    Ok(())
}
