mod cli;
mod mode;
mod parser;
mod transport;

// Re-export from ailoop-core
// use ailoop_core::*; // Not used in main.rs currently

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::handlers;

#[derive(Parser)]
#[command(name = "ailoop")]
#[command(version = "0.1.7")]
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
    /// Ask a question and collect human response
    ///
    /// Supports both text questions and multiple choice questions.
    ///
    /// Multiple Choice Format:
    ///   Use pipe (|) separator: "question|choice1|choice2|choice3"
    ///   Example: "What color?|red|blue|green"
    ///
    /// JSON Response Format (with --json):
    ///   For text questions:
    ///     {"response": "answer text", "channel": "public", "timestamp": "..."}
    ///
    ///   For multiple choice:
    ///     {
    ///       "response": "selected_choice_text",
    ///       "channel": "public",
    ///       "timestamp": "...",
    ///       "metadata": {
    ///         "index": 0,     // 0-based index of selected choice
    ///         "value": "red"  // Selected choice value
    ///       }
    ///     }
    ///
    /// Examples:
    ///   ailoop ask "What is your name?"
    ///   ailoop ask "Choose a color|red|blue|green" --server http://localhost:8080
    ///   ailoop ask "Select option|option1|option2" --json --timeout 60
    Ask {
        /// The question text. For multiple choice, use pipe separator: "question|choice1|choice2|..."
        question: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Response timeout in seconds (0 = no timeout)
        #[arg(short, long, default_value = "0")]
        timeout: u32,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format. For multiple choice, includes 'index' and 'value' in metadata
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

    /// Workflow orchestration commands
    Workflow {
        #[command(subcommand)]
        command: cli::workflow::WorkflowCommands,
    },

    /// Task management commands
    Task {
        #[command(subcommand)]
        command: cli::task::TaskCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ask {
            question,
            channel,
            timeout,
            server,
            json,
        } => {
            handlers::handle_ask(question, channel, timeout, server, json).await?;
        }
        Commands::Authorize {
            action,
            channel,
            timeout,
            server,
            json,
        } => {
            handlers::handle_authorize(action, channel, timeout, server, json).await?;
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
        } => {
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
        Commands::Workflow { command } => {
            use cli::workflow::WorkflowCommands;
            use cli::workflow_handlers;

            match command {
                WorkflowCommands::Start {
                    workflow_name,
                    initiator,
                    json,
                } => {
                    workflow_handlers::handle_workflow_start(workflow_name, initiator, json)
                        .await?;
                }
                WorkflowCommands::Status { execution_id, json } => {
                    workflow_handlers::handle_workflow_status(execution_id, json).await?;
                }
                WorkflowCommands::List { json } => {
                    workflow_handlers::handle_workflow_list(json).await?;
                }
                WorkflowCommands::History { workflow, json } => {
                    workflow_handlers::handle_workflow_history(workflow, json).await?;
                }
                WorkflowCommands::Approve {
                    approval_id,
                    operator,
                    json,
                } => {
                    workflow_handlers::handle_workflow_approve(approval_id, operator, json).await?;
                }
                WorkflowCommands::Deny {
                    approval_id,
                    operator,
                    json,
                } => {
                    workflow_handlers::handle_workflow_deny(approval_id, operator, json).await?;
                }
                WorkflowCommands::ListApprovals { execution, json } => {
                    workflow_handlers::handle_workflow_list_approvals(execution, json).await?;
                }
                WorkflowCommands::Logs {
                    execution_id,
                    state,
                    limit,
                    offset,
                    follow,
                    json,
                } => {
                    workflow_handlers::handle_workflow_logs(
                        execution_id,
                        state,
                        limit,
                        offset,
                        follow,
                        json,
                    )
                    .await?;
                }
                WorkflowCommands::Metrics { workflow, json } => {
                    workflow_handlers::handle_workflow_metrics(workflow, json).await?;
                }
                WorkflowCommands::Validate {
                    workflow_file,
                    json,
                } => {
                    workflow_handlers::handle_workflow_validate(workflow_file, json).await?;
                }
                WorkflowCommands::ListDefs { directory, json } => {
                    workflow_handlers::handle_workflow_list_defs(directory, json).await?;
                }
            }
        }
        Commands::Task { command } => {
            crate::cli::task_handlers::handle_task_commands(command).await?;
        }
    }

    Ok(())
}
