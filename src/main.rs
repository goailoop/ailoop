mod cli;
mod server;
mod channel;
mod models;
mod services;
mod transport;
mod parser;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::handlers;

#[derive(Parser)]
#[command(name = "ailoop")]
#[command(about = "Human-in-the-Loop CLI Tool for AI Agent Communication")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ask a question and collect human response
    Ask {
        /// The question text
        question: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Response timeout in seconds (0 = no timeout)
        #[arg(short, long, default_value = "0")]
        timeout: u32,

        /// Server URL for remote operation
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server: String,

        /// Output in JSON format
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
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
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
        #[arg(long, default_value = "http://127.0.0.1:8080")]
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
        #[arg(long, default_value = "http://127.0.0.1:8080")]
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
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server: String,
    },

    /// Forward agent output to ailoop server
    Forward {
        /// Channel name for messages
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Agent type (cursor, jsonl, or auto-detect)
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ask { question, channel, timeout, server, json } => {
            handlers::handle_ask(question, channel, timeout, server, json).await?;
        }
        Commands::Authorize { action, channel, timeout, server, json } => {
            handlers::handle_authorize(action, channel, timeout, server, json).await?;
        }
        Commands::Say { message, channel, priority, server } => {
            handlers::handle_say(message, channel, priority, server).await?;
        }
        Commands::Serve { host, port, channel } => {
            handlers::handle_serve(host, port, channel).await?;
        }
        Commands::Config { init, config_file } => {
            if init {
                handlers::handle_config_init(config_file).await?;
            } else {
                println!("Config command requires --init flag");
                println!("Usage: ailoop config --init [--config-file PATH]");
            }
        }
        Commands::Image { image_path, channel, server } => {
            handlers::handle_image(image_path, channel, server).await?;
        }
        Commands::Navigate { url, channel, server } => {
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
                channel,
                agent_type,
                format,
                transport,
                url,
                output,
                client_id,
                input,
            )
            .await?;
        }
    }

    Ok(())
}