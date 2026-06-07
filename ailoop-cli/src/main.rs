mod cli;
mod mode;
mod parser;

use anyhow::Result;
use cli_framework::prelude::*;
use cli_framework::spec::arg_spec::{ArgKind, ArgSpec, ArgValueType, Cardinality};
use cli_framework::spec::command_tree::GroupMetadata;
use std::collections::HashMap;
use std::sync::Arc;

struct AiloopApp;
impl AppContext for AiloopApp {}

// ── arg extraction helpers ─────────────────────────────────────────────────────

fn named(args: &HashMap<String, ArgValue>, key: &str) -> String {
    match args.get(key) {
        Some(ArgValue::Str(s)) => s.clone(),
        _ => String::new(),
    }
}

fn named_or(args: &HashMap<String, ArgValue>, key: &str, default: &str) -> String {
    match args.get(key) {
        Some(ArgValue::Str(s)) if !s.is_empty() => s.clone(),
        _ => default.to_string(),
    }
}

fn flag(args: &HashMap<String, ArgValue>, key: &str) -> bool {
    matches!(args.get(key), Some(ArgValue::Bool(true)))
}

fn opt_named(args: &HashMap<String, ArgValue>, key: &str) -> Option<String> {
    match args.get(key) {
        Some(ArgValue::Str(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

// ── arg spec helpers ───────────────────────────────────────────────────────────

fn opt_arg(name: &'static str, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind: ArgKind::Option,
        short: None,
        long: None,
        value_type: ArgValueType::String,
        cardinality: Cardinality::Optional,
        default: None,
        conflicts_with: vec![],
        requires: vec![],
        help,
        ..Default::default()
    }
}

fn opt_arg_default(name: &'static str, default: &'static str, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind: ArgKind::Option,
        short: None,
        long: None,
        value_type: ArgValueType::String,
        cardinality: Cardinality::Optional,
        default: Some(ArgValue::Str(default.to_string())),
        conflicts_with: vec![],
        requires: vec![],
        help,
        ..Default::default()
    }
}

fn req_opt_arg(name: &'static str, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind: ArgKind::Option,
        short: None,
        long: None,
        value_type: ArgValueType::String,
        cardinality: Cardinality::Required,
        default: None,
        conflicts_with: vec![],
        requires: vec![],
        help,
        ..Default::default()
    }
}

fn req_pos_arg(name: &'static str, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind: ArgKind::Positional,
        short: None,
        long: None,
        value_type: ArgValueType::String,
        cardinality: Cardinality::Required,
        default: None,
        conflicts_with: vec![],
        requires: vec![],
        help,
        ..Default::default()
    }
}

fn flag_arg(name: &'static str, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind: ArgKind::Flag,
        short: None,
        long: None,
        value_type: ArgValueType::Bool,
        cardinality: Cardinality::Optional,
        default: None,
        conflicts_with: vec![],
        requires: vec![],
        help,
        ..Default::default()
    }
}

fn channel_arg() -> ArgSpec {
    opt_arg_default("channel", "public", "Channel name")
}

fn server_arg() -> ArgSpec {
    opt_arg("server", "Server URL for remote operation")
}

fn json_arg() -> ArgSpec {
    flag_arg("json", "Output in JSON format")
}

// ── command factories ──────────────────────────────────────────────────────────

fn ask_command() -> Command {
    Command {
        id: "ask".into(),
        spec: Arc::new(CommandSpec {
            summary: "Send a structured decision and collect human selection",
            syntax: Some("ask --payload <JSON>"),
            category: Some("human-in-the-loop"),
            args: vec![
                req_opt_arg(
                    "payload",
                    "JSON-encoded decision payload (decision_id, summary, options, ...)",
                ),
                channel_arg(),
                opt_arg_default(
                    "timeout",
                    "0",
                    "Response timeout in seconds (0 = use payload timeout)",
                ),
                server_arg(),
                json_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let payload = named(&args, "payload");
                let channel = named_or(&args, "channel", "public");
                let timeout: u32 = named_or(&args, "timeout", "0").parse().unwrap_or(0);
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::handlers::handle_ask(payload, channel, timeout, server, json).await
            })
        }),
    }
}

fn authorize_command() -> Command {
    Command {
        id: "authorize".into(),
        spec: Arc::new(CommandSpec {
            summary: "Request authorization for a critical action",
            syntax: Some("authorize <action>"),
            category: Some("human-in-the-loop"),
            args: vec![
                req_pos_arg("action", "Description of action requiring authorization"),
                channel_arg(),
                opt_arg_default("timeout", "300", "Authorization timeout in seconds"),
                server_arg(),
                json_arg(),
                opt_arg_default(
                    "default",
                    "yes",
                    "Default decision when ENTER is pressed (yes or no)",
                ),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let action = named(&args, "action");
                let channel = named_or(&args, "channel", "public");
                let timeout: u32 = named_or(&args, "timeout", "300").parse().unwrap_or(300);
                let server = named(&args, "server");
                let json = flag(&args, "json");
                let default_yes = named_or(&args, "default", "yes") != "no";
                cli::handlers::handle_authorize(action, channel, timeout, server, json, default_yes)
                    .await
            })
        }),
    }
}

fn say_command() -> Command {
    Command {
        id: "say".into(),
        spec: Arc::new(CommandSpec {
            summary: "Send a notification message",
            syntax: Some("say <message>"),
            category: Some("human-in-the-loop"),
            args: vec![
                req_pos_arg("message", "Notification message text"),
                channel_arg(),
                opt_arg_default(
                    "priority",
                    "normal",
                    "Message priority (normal, high, critical)",
                ),
                server_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let message = named(&args, "message");
                let channel = named_or(&args, "channel", "public");
                let priority = named_or(&args, "priority", "normal");
                let server = named(&args, "server");
                cli::handlers::handle_say(message, channel, priority, server).await
            })
        }),
    }
}

fn serve_command() -> Command {
    Command {
        id: "serve".into(),
        spec: Arc::new(CommandSpec {
            summary: "Start ailoop server for multi-agent communication",
            syntax: Some("serve [--host HOST] [--port PORT]"),
            category: Some("server"),
            args: vec![
                opt_arg_default("host", "127.0.0.1", "Server bind address"),
                opt_arg_default("port", "8080", "Server port number"),
                channel_arg(),
                flag_arg(
                    "web",
                    "Enable the embedded web UI on the HTTP API port (port+1)",
                ),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: false,
        expose_chat: false,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let host = named_or(&args, "host", "127.0.0.1");
                let port: u16 = named_or(&args, "port", "8080").parse().unwrap_or(8080);
                let channel = named_or(&args, "channel", "public");
                let web = flag(&args, "web");
                cli::handlers::handle_serve(host, port, channel, web).await
            })
        }),
    }
}

fn config_command() -> Command {
    Command {
        id: "config".into(),
        spec: Arc::new(CommandSpec {
            summary: "Configure ailoop settings",
            syntax: Some("config [--init] [--config-file PATH]"),
            category: Some("configuration"),
            args: vec![
                flag_arg("init", "Start interactive configuration setup"),
                opt_arg_default(
                    "config-file",
                    "~/.config/ailoop/config.toml",
                    "Path to configuration file",
                ),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: false,
        expose_chat: false,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let init = flag(&args, "init");
                let config_file = named_or(&args, "config-file", "~/.config/ailoop/config.toml");
                if init {
                    cli::handlers::handle_config_init(config_file).await
                } else {
                    cli::handlers::handle_config_show(config_file).await
                }
            })
        }),
    }
}

fn image_command() -> Command {
    Command {
        id: "image".into(),
        spec: Arc::new(CommandSpec {
            summary: "Display an image to the user",
            syntax: Some("image <path>"),
            category: Some("media"),
            args: vec![
                req_pos_arg("image_path", "Image file path or URL"),
                channel_arg(),
                server_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let image_path = named(&args, "image_path");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                cli::handlers::handle_image(image_path, channel, server).await
            })
        }),
    }
}

fn navigate_command() -> Command {
    Command {
        id: "navigate".into(),
        spec: Arc::new(CommandSpec {
            summary: "Suggest user to navigate to a URL",
            syntax: Some("navigate <url>"),
            category: Some("media"),
            args: vec![
                req_pos_arg("url", "URL to navigate to"),
                channel_arg(),
                server_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let url = named(&args, "url");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                cli::handlers::handle_navigate(url, channel, server).await
            })
        }),
    }
}

fn forward_command() -> Command {
    Command {
        id: "forward".into(),
        spec: Arc::new(CommandSpec {
            summary: "Forward agent output to ailoop server",
            syntax: Some("forward [--channel CHANNEL] [--agent-type TYPE]"),
            category: Some("agent"),
            args: vec![
                channel_arg(),
                opt_arg(
                    "agent-type",
                    "Agent type (cursor, jsonl, opencode, or auto-detect)",
                ),
                opt_arg_default(
                    "format",
                    "stream-json",
                    "Input format (json, stream-json, text)",
                ),
                opt_arg_default("transport", "websocket", "Transport type (websocket, file)"),
                opt_arg_default("url", "ws://127.0.0.1:8080", "WebSocket server URL"),
                opt_arg("output", "Output file path (for file transport)"),
                opt_arg("client-id", "Client ID for tracking"),
                opt_arg("input", "Input file path (if not reading from stdin)"),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: false,
        expose_chat: false,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let channel = named_or(&args, "channel", "public");
                let agent_type = opt_named(&args, "agent-type");
                let format = named_or(&args, "format", "stream-json");
                let transport = named_or(&args, "transport", "websocket");
                let url = Some(named_or(&args, "url", "ws://127.0.0.1:8080"));
                let output = opt_named(&args, "output");
                let client_id = opt_named(&args, "client-id");
                let input = opt_named(&args, "input");
                cli::handlers::handle_forward(
                    channel, agent_type, format, transport, url, output, client_id, input,
                )
                .await
            })
        }),
    }
}

fn queue_command() -> Command {
    Command {
        id: "queue".into(),
        spec: Arc::new(CommandSpec {
            summary: "Inspect the human prompt queue",
            syntax: Some("queue [--channel CHANNEL]"),
            category: Some("human-in-the-loop"),
            args: vec![
                server_arg(),
                opt_arg("channel", "Filter by channel name"),
                json_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: false,
        expose_chat: false,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let server = named(&args, "server");
                let channel = opt_named(&args, "channel");
                let json = flag(&args, "json");
                cli::queue_handlers::handle_queue(server, channel, json).await
            })
        }),
    }
}

// ── task subcommands ───────────────────────────────────────────────────────────

fn task_create_command() -> Command {
    Command {
        id: "create".into(),
        spec: Arc::new(CommandSpec {
            summary: "Create a new task",
            syntax: Some("task create <title> --description DESC"),
            category: Some("task"),
            args: vec![
                req_pos_arg("title", "Task title"),
                req_opt_arg("description", "Detailed task description"),
                channel_arg(),
                server_arg(),
                json_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let title = named(&args, "title");
                let description = named(&args, "description");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::task_handlers::handle_task_create(title, description, channel, server, json)
                    .await
            })
        }),
    }
}

fn task_list_command() -> Command {
    Command {
        id: "list".into(),
        spec: Arc::new(CommandSpec {
            summary: "List all tasks",
            syntax: Some("task list [--state STATE]"),
            category: Some("task"),
            args: vec![
                channel_arg(),
                opt_arg("state", "Filter by task state (pending, done, abandoned)"),
                server_arg(),
                json_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let channel = named_or(&args, "channel", "public");
                let state = opt_named(&args, "state");
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::task_handlers::handle_task_list(channel, state, server, json).await
            })
        }),
    }
}

fn task_show_command() -> Command {
    Command {
        id: "show".into(),
        spec: Arc::new(CommandSpec {
            summary: "Show task details",
            syntax: Some("task show <task_id>"),
            category: Some("task"),
            args: vec![
                req_pos_arg("task_id", "Task ID"),
                channel_arg(),
                server_arg(),
                json_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let task_id = named(&args, "task_id");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::task_handlers::handle_task_show(task_id, channel, server, json).await
            })
        }),
    }
}

fn task_update_command() -> Command {
    Command {
        id: "update".into(),
        spec: Arc::new(CommandSpec {
            summary: "Update task state",
            syntax: Some("task update <task_id> --state STATE"),
            category: Some("task"),
            args: vec![
                req_pos_arg("task_id", "Task ID"),
                req_opt_arg("state", "New task state (pending, done, abandoned)"),
                channel_arg(),
                server_arg(),
                json_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let task_id = named(&args, "task_id");
                let state = named(&args, "state");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::task_handlers::handle_task_update(task_id, state, channel, server, json).await
            })
        }),
    }
}

fn task_ready_command() -> Command {
    Command {
        id: "ready".into(),
        spec: Arc::new(CommandSpec {
            summary: "List tasks ready to start (no blockers)",
            syntax: Some("task ready"),
            category: Some("task"),
            args: vec![channel_arg(), server_arg(), json_arg()],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::task_handlers::handle_task_ready(channel, server, json).await
            })
        }),
    }
}

fn task_blocked_command() -> Command {
    Command {
        id: "blocked".into(),
        spec: Arc::new(CommandSpec {
            summary: "List blocked tasks",
            syntax: Some("task blocked"),
            category: Some("task"),
            args: vec![channel_arg(), server_arg(), json_arg()],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                let json = flag(&args, "json");
                cli::task_handlers::handle_task_blocked(channel, server, json).await
            })
        }),
    }
}

fn task_dep_add_command() -> Command {
    Command {
        id: "add".into(),
        spec: Arc::new(CommandSpec {
            summary: "Add a dependency between tasks",
            syntax: Some("task dep add <child_id> <parent_id>"),
            category: Some("task"),
            args: vec![
                req_pos_arg("child_id", "Child task ID"),
                req_pos_arg("parent_id", "Parent task ID"),
                opt_arg_default(
                    "dependency-type",
                    "blocks",
                    "Dependency type (blocks, related, parent)",
                ),
                channel_arg(),
                server_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let child_id = named(&args, "child_id");
                let parent_id = named(&args, "parent_id");
                let dependency_type = named_or(&args, "dependency-type", "blocks");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                cli::task_handlers::handle_dep_add(
                    child_id,
                    parent_id,
                    dependency_type,
                    channel,
                    server,
                )
                .await
            })
        }),
    }
}

fn task_dep_remove_command() -> Command {
    Command {
        id: "remove".into(),
        spec: Arc::new(CommandSpec {
            summary: "Remove a dependency between tasks",
            syntax: Some("task dep remove <child_id> <parent_id>"),
            category: Some("task"),
            args: vec![
                req_pos_arg("child_id", "Child task ID"),
                req_pos_arg("parent_id", "Parent task ID"),
                channel_arg(),
                server_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let child_id = named(&args, "child_id");
                let parent_id = named(&args, "parent_id");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                cli::task_handlers::handle_dep_remove(child_id, parent_id, channel, server).await
            })
        }),
    }
}

fn task_dep_graph_command() -> Command {
    Command {
        id: "graph".into(),
        spec: Arc::new(CommandSpec {
            summary: "Show dependency graph for a task",
            syntax: Some("task dep graph <task_id>"),
            category: Some("task"),
            args: vec![
                req_pos_arg("task_id", "Task ID"),
                channel_arg(),
                server_arg(),
            ],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: true,
        expose_chat: true,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let task_id = named(&args, "task_id");
                let channel = named_or(&args, "channel", "public");
                let server = named(&args, "server");
                cli::task_handlers::handle_dep_graph(task_id, channel, server).await
            })
        }),
    }
}

// ── provider subcommands ───────────────────────────────────────────────────────

fn provider_list_command() -> Command {
    Command {
        id: "list".into(),
        spec: Arc::new(CommandSpec {
            summary: "List configured providers and their status",
            syntax: Some("provider list"),
            category: Some("provider"),
            args: vec![opt_arg_default(
                "config",
                "~/.config/ailoop/config.toml",
                "Path to config file",
            )],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: false,
        expose_chat: false,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let config = named_or(&args, "config", "~/.config/ailoop/config.toml");
                cli::provider_handlers::handle_provider_list(&config).await
            })
        }),
    }
}

fn provider_telegram_test_command() -> Command {
    Command {
        id: "test".into(),
        spec: Arc::new(CommandSpec {
            summary: "Send a test message to the configured Telegram chat",
            syntax: Some("provider telegram test"),
            category: Some("provider"),
            args: vec![opt_arg_default(
                "config",
                "~/.config/ailoop/config.toml",
                "Path to config file",
            )],
            ..Default::default()
        }),
        validator: None,
        expose_mcp: false,
        expose_chat: false,
        execute: Arc::new(|_ctx, args| {
            Box::pin(async move {
                let config = named_or(&args, "config", "~/.config/ailoop/config.toml");
                cli::provider_handlers::handle_provider_telegram_test(&config).await
            })
        }),
    }
}

// ── doctor checks ──────────────────────────────────────────────────────────────

fn ailoop_doctor_checks() -> Vec<Arc<dyn cli_framework::doctor::check::DoctorCheck>> {
    vec![
        Arc::new(cli::doctor::ConfigFileCheck),
        Arc::new(cli::doctor::ServerConnectivityCheck),
    ]
}

// ── entry point ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let task_path = |segs: &[&str]| CommandPath::new(segs).expect("valid path");

    let mut app = AppBuilder::new()
        .with_version("ailoop", env!("CARGO_PKG_VERSION"))
        .with_ailoop_channel("public")
        // human-in-the-loop
        .register_command(ask_command())?
        .register_command(authorize_command())?
        .register_command(say_command())?
        // server
        .register_command(serve_command())?
        // configuration
        .register_command(config_command())?
        // media
        .register_command(image_command())?
        .register_command(navigate_command())?
        // agent
        .register_command(forward_command())?
        // queue
        .register_command(queue_command())?
        // task group
        .register_group(
            &CommandPath::root_for("task"),
            GroupMetadata {
                summary: "Task management commands",
                hidden: false,
            },
        )?
        .register_command_at(&task_path(&["task", "create"]), task_create_command())?
        .register_command_at(&task_path(&["task", "list"]), task_list_command())?
        .register_command_at(&task_path(&["task", "show"]), task_show_command())?
        .register_command_at(&task_path(&["task", "update"]), task_update_command())?
        .register_command_at(&task_path(&["task", "ready"]), task_ready_command())?
        .register_command_at(&task_path(&["task", "blocked"]), task_blocked_command())?
        // task dep sub-group
        .register_group(
            &task_path(&["task", "dep"]),
            GroupMetadata {
                summary: "Manage task dependencies",
                hidden: false,
            },
        )?
        .register_command_at(&task_path(&["task", "dep", "add"]), task_dep_add_command())?
        .register_command_at(
            &task_path(&["task", "dep", "remove"]),
            task_dep_remove_command(),
        )?
        .register_command_at(
            &task_path(&["task", "dep", "graph"]),
            task_dep_graph_command(),
        )?
        // provider group
        .register_group(
            &CommandPath::root_for("provider"),
            GroupMetadata {
                summary: "Provider status and testing",
                hidden: false,
            },
        )?
        .register_command_at(&task_path(&["provider", "list"]), provider_list_command())?
        .register_group(
            &task_path(&["provider", "telegram"]),
            GroupMetadata {
                summary: "Telegram provider commands",
                hidden: false,
            },
        )?
        .register_command_at(
            &task_path(&["provider", "telegram", "test"]),
            provider_telegram_test_command(),
        )?
        // doctor checks (auto-registers `doctor` command)
        .register_doctor_checks(ailoop_doctor_checks())
        .build(AiloopApp)?;

    app.run().await
}
