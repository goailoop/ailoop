//! Main server integration for ailoop

use crate::server::providers::{
    resolve_effective_timeout, PendingPromptRegistry, PromptType, ReplySource,
};
use ailoop_core::channel::ChannelIsolation;
use ailoop_core::models::{Configuration, Message, MessageContent, ResponseType};
use ailoop_core::terminal::countdown::CountdownRenderer;
use anyhow::{Context, Result};
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use std::future::Future;
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::time::{interval, Duration};

/// Shared state passed to every Axum handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub message_history: Arc<crate::server::history::MessageHistory>,
    pub broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    pub task_storage: Arc<ailoop_core::server::TaskStorage>,
    pub pending_prompt_registry: Arc<PendingPromptRegistry>,
    pub channel_manager: Arc<ChannelIsolation>,
    pub default_channel: String,
    pub web: bool,
}

/// Main ailoop server
pub struct AiloopServer {
    host: String,
    port: u16,
    default_channel: String,
    channel_manager: Arc<ChannelIsolation>,
    message_history: Arc<crate::server::history::MessageHistory>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    task_storage: Arc<ailoop_core::server::TaskStorage>,
    pending_prompt_registry: Arc<PendingPromptRegistry>,
    config: Option<Configuration>,
    /// Whether to serve the embedded web UI on the HTTP port
    web: bool,
}

/// Server status for UI
#[derive(Clone, Debug)]
pub struct ServerStatus {
    pub status: String,
    pub total_queue_size: usize,
    pub total_connections: usize,
    pub active_channels: usize,
}

impl AiloopServer {
    /// Create a new ailoop server
    pub fn new(host: String, port: u16, default_channel: String) -> Self {
        let channel_manager = Arc::new(ChannelIsolation::new(default_channel.clone()));
        let message_history = Arc::new(crate::server::history::MessageHistory::new());
        let broadcast_manager = Arc::new(crate::server::broadcast::BroadcastManager::new());
        let task_storage = Arc::new(ailoop_core::server::TaskStorage::new());
        let pending_prompt_registry = Arc::new(PendingPromptRegistry::new());

        Self {
            host,
            port,
            default_channel,
            channel_manager,
            message_history,
            broadcast_manager,
            task_storage,
            pending_prompt_registry,
            config: None,
            web: false,
        }
    }

    /// Attach configuration (for provider loading at startup).
    pub fn with_config(mut self, config: Configuration) -> Self {
        self.config = Some(config);
        self
    }

    /// Enable the embedded web UI served on the HTTP port.
    pub fn with_web(mut self, enable: bool) -> Self {
        self.web = enable;
        self
    }

    /// Start the server
    pub async fn start(self) -> Result<()> {
        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        self.start_with_shutdown(shutdown).await
    }

    /// Start the server until the provided shutdown future completes.
    pub async fn start_with_shutdown<F>(self, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        use std::net::SocketAddr;
        let address: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid server address")?;

        println!("ailoop server starting on {}", address);
        println!("Default channel: {}", self.default_channel);
        println!("Press Ctrl+C to stop the server");
        println!("Starting server initialization...");

        // Provider config: register Telegram sink and reply source when enabled
        if let Some(ref cfg) = self.config {
            if cfg.providers.telegram.enabled {
                let token = std::env::var("AILOOP_TELEGRAM_BOT_TOKEN").ok();
                let chat_id = cfg
                    .providers
                    .telegram
                    .chat_id
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .cloned();
                match (token, chat_id) {
                    (Some(t), Some(c)) => {
                        match crate::server::providers::TelegramSink::new(t.clone(), c) {
                            Ok(sink) => {
                                self.broadcast_manager
                                    .add_notification_sink(Arc::new(sink))
                                    .await;
                                let reply_source: Arc<dyn ReplySource> =
                                    Arc::new(crate::server::providers::TelegramReplySource::new(t));
                                let registry = Arc::clone(&self.pending_prompt_registry);
                                tokio::spawn(async move {
                                    loop {
                                        if let Some(reply) = reply_source.next_reply().await {
                                            registry
                                                .submit_reply(
                                                    reply.reply_to_message_id,
                                                    reply.answer,
                                                    reply.response_type,
                                                )
                                                .await;
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to create Telegram sink: {}", e);
                            }
                        }
                    }
                    (None, _) => {
                        tracing::warn!("Telegram provider skipped: token not set");
                    }
                    (_, None) => {
                        tracing::warn!("Telegram provider skipped: chat_id not configured");
                    }
                }
            }
        }

        if self.web {
            println!("Web UI available at http://{}:{}/", self.host, self.port);
        }

        let app_state = AppState {
            message_history: Arc::clone(&self.message_history),
            broadcast_manager: Arc::clone(&self.broadcast_manager),
            task_storage: Arc::clone(&self.task_storage),
            pending_prompt_registry: Arc::clone(&self.pending_prompt_registry),
            channel_manager: Arc::clone(&self.channel_manager),
            default_channel: self.default_channel.clone(),
            web: self.web,
        };

        let router = create_server_router(app_state, self.web);
        println!("API routes created");

        // Spawn message processing task
        let channel_manager_msg = Arc::clone(&self.channel_manager);
        let broadcast_manager_msg = Arc::clone(&self.broadcast_manager);
        let pending_registry = Arc::clone(&self.pending_prompt_registry);
        let config_for_tasks = self.config.clone();
        let message_task = tokio::spawn(async move {
            Self::process_queued_messages(
                channel_manager_msg,
                broadcast_manager_msg,
                pending_registry,
                config_for_tasks,
            )
            .await;
        });

        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", address, e))?;

        axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(shutdown)
            .await?;

        message_task.abort();
        println!("\nShutting down server...");
        Ok(())
    }

    /// Handle a single WebSocket connection upgraded by Axum
    pub(crate) async fn handle_ws_connection_inner(
        ws: WebSocket,
        channel_manager: Arc<ChannelIsolation>,
        default_channel: String,
        message_history: Arc<crate::server::history::MessageHistory>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    ) {
        let (mut ws_sender, mut ws_receiver) = ws.split();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<WsMessage>();
        let tx_replay = tx.clone();
        let mut channel_name = default_channel.clone();

        // Connections start as Agent; the browser sends a hello frame to become a Viewer
        let connection_type = crate::server::broadcast::ConnectionType::Agent;
        let connection_id = broadcast_manager.add_viewer(connection_type, tx).await;

        // Track connection
        channel_manager.add_connection(&channel_name);

        // Forward outgoing messages to the WebSocket
        let forward_task = tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if SinkExt::send(&mut ws_sender, msg).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages
        let mut is_viewer = false;
        while let Some(msg) = ws_receiver.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            };

            if matches!(msg, WsMessage::Close(_)) {
                break;
            }

            let text = if let WsMessage::Text(t) = msg {
                t
            } else {
                continue;
            };

            // Check for viewer hello frame: {"subscribe": "*"} or {"subscribe": [...]}
            if !is_viewer {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if val.get("subscribe").is_some() {
                        is_viewer = true;
                        broadcast_manager.set_viewer_mode(&connection_id).await.ok();
                        broadcast_manager
                            .subscribe_to_all(&connection_id)
                            .await
                            .ok();
                        // Replay history so the page is not blank on connect
                        let channels = message_history.get_channels().await;
                        for ch in channels {
                            let msgs = message_history.get_messages(&ch, Some(500)).await;
                            for m in msgs {
                                if let Ok(j) = serde_json::to_string(&m) {
                                    let _ = tx_replay.send(WsMessage::Text(j));
                                }
                            }
                        }
                        continue;
                    }
                }
            }

            if is_viewer {
                // Viewers are read-only; they submit responses via HTTP API
                continue;
            }

            // Agent path: parse and enqueue the message
            match serde_json::from_str::<Message>(&text) {
                Ok(message) => {
                    channel_name = message.channel.clone();

                    let broadcast_clone = Arc::clone(&broadcast_manager);
                    let connection_id_clone = connection_id;
                    let channel_clone = channel_name.clone();
                    if let Err(e) = broadcast_clone
                        .subscribe_to_channel(&connection_id_clone, &channel_clone)
                        .await
                    {
                        eprintln!("Failed to subscribe to channel: {}", e);
                    }

                    let history_clone = Arc::clone(&message_history);
                    let broadcast_clone2 = Arc::clone(&broadcast_manager);
                    let channel_clone2 = channel_name.clone();
                    let message_clone = message.clone();
                    let is_interactive = matches!(
                        message_clone.content,
                        MessageContent::Decision { .. }
                            | MessageContent::Authorization { .. }
                            | MessageContent::Navigate { .. }
                    );
                    tokio::spawn(async move {
                        history_clone
                            .add_message(&channel_clone2, message_clone.clone())
                            .await;
                        if is_interactive {
                            broadcast_clone2
                                .broadcast_to_viewers_only(&message_clone)
                                .await;
                        } else {
                            broadcast_clone2.broadcast_message(&message_clone).await;
                        }
                    });

                    channel_manager.enqueue_message(&channel_name, message);
                }
                Err(e) => {
                    eprintln!("Failed to parse message: {}", e);
                }
            }
        }

        // Cleanup
        forward_task.abort();
        broadcast_manager.remove_viewer(&connection_id).await;
        if !is_viewer {
            channel_manager.remove_connection(&channel_name);
        }
    }

    /// Process queued messages and display them to users
    async fn process_queued_messages(
        channel_manager: Arc<ChannelIsolation>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
        config: Option<Configuration>,
    ) {
        let mut check_interval = interval(Duration::from_millis(100));

        loop {
            check_interval.tick().await;

            let active_channels = channel_manager.get_active_channels();

            for channel_name in active_channels {
                if let Some(message) = channel_manager.dequeue_message(&channel_name) {
                    println!("\nProcessing message from queue [{}]", channel_name);

                    let response_type = match &message.content {
                        MessageContent::Decision {
                            decision_id,
                            summary,
                            context_markdown,
                            options,
                            recommendation,
                            timeout_seconds,
                        } => {
                            let broadcast_clone = Arc::clone(&broadcast_manager);
                            let registry = Arc::clone(&pending_registry);
                            Self::handle_decision(
                                message.clone(),
                                decision_id.clone(),
                                summary.clone(),
                                context_markdown.clone(),
                                options.clone(),
                                recommendation.clone(),
                                *timeout_seconds,
                                broadcast_clone,
                                registry,
                                config.as_ref(),
                            )
                            .await
                        }
                        MessageContent::Authorization {
                            action,
                            timeout_seconds,
                            ..
                        } => {
                            let broadcast_clone = Arc::clone(&broadcast_manager);
                            let registry = Arc::clone(&pending_registry);
                            Self::handle_authorization(
                                message.clone(),
                                action.clone(),
                                *timeout_seconds,
                                broadcast_clone,
                                registry,
                                config.as_ref(),
                            )
                            .await
                        }
                        MessageContent::Notification { text, priority } => {
                            Self::handle_notification(text.clone(), priority.clone());
                            ResponseType::Text
                        }
                        MessageContent::Navigate { url } => {
                            let broadcast_clone = Arc::clone(&broadcast_manager);
                            let registry = Arc::clone(&pending_registry);
                            Self::handle_navigate(
                                message.clone(),
                                url.clone(),
                                broadcast_clone,
                                registry,
                                config.as_ref(),
                            )
                            .await
                        }
                        _ => ResponseType::Text,
                    };

                    if matches!(response_type, ResponseType::Cancelled) {
                        channel_manager.enqueue_message(&channel_name, message);
                    }
                }
            }
        }
    }

    /// Resolve a human answer string to a canonical decision option id.
    /// Returns (option_id, label, 0-based index) on success, None if no match.
    fn resolve_decision_answer(
        input: &str,
        options: &[ailoop_core::models::DecisionOption],
    ) -> Option<(String, String, usize)> {
        let trimmed = input.trim();
        // 1. Exact match against option id
        if let Some((idx, opt)) = options.iter().enumerate().find(|(_, o)| o.id == trimmed) {
            return Some((opt.id.clone(), opt.label.clone(), idx));
        }
        // 2. Case-insensitive match against option label
        if let Some((idx, opt)) = options
            .iter()
            .enumerate()
            .find(|(_, o)| o.label.eq_ignore_ascii_case(trimmed))
        {
            return Some((opt.id.clone(), opt.label.clone(), idx));
        }
        // 3. 1-based integer index
        if let Ok(n) = trimmed.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                let idx = n - 1;
                let opt = &options[idx];
                return Some((opt.id.clone(), opt.label.clone(), idx));
            }
        }
        None
    }

    /// Handle a structured decision message. First valid response (terminal or provider) wins.
    /// On DECISION_UNKNOWN_ANSWER, re-prompts without dropping the pending session.
    #[allow(clippy::too_many_arguments)]
    async fn handle_decision(
        message: Message,
        decision_id: String,
        summary: String,
        _context_markdown: Option<String>,
        options: Vec<ailoop_core::models::DecisionOption>,
        recommendation: Option<ailoop_core::models::DecisionRecommendation>,
        timeout_secs: u32,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
        config: Option<&Configuration>,
    ) -> ResponseType {
        let use_terminal = io::stdin().is_terminal() && io::stdout().is_terminal();

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "Decision [{}] ({}): {}",
            message.channel, decision_id, summary
        );
        if timeout_secs > 0 {
            println!("Timeout: {} seconds", timeout_secs);
        }
        println!("\nOptions:");
        for (idx, opt) in options.iter().enumerate() {
            let rec_marker = recommendation
                .as_ref()
                .filter(|r| r.option_id == opt.id)
                .map(|_| " [recommended]")
                .unwrap_or("");
            if let Some(detail) = &opt.detail_markdown {
                let truncated: String = detail.chars().take(80).collect();
                println!("  {}. {}{} — {}", idx + 1, opt.label, rec_marker, truncated);
            } else {
                println!("  {}. {}{}", idx + 1, opt.label, rec_marker);
            }
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        if use_terminal {
            print!("Enter option id, label, or number (ESC to skip): ");
            let _ = io::stdout().flush();
        }

        let reply_to_id = broadcast_manager
            .send_to_notification_sinks_and_get_reply_to_id(&message)
            .await;

        let timeout_duration = resolve_effective_timeout(timeout_secs, config);

        // Loop: re-register on each attempt so rx is always fresh.
        // Re-prompts on DECISION_UNKNOWN_ANSWER without timing out.
        let (resolved_id, resolved_label, resolved_index, response_type) = loop {
            let (rx, completer) = pending_registry
                .register(message.id, reply_to_id.clone(), PromptType::Decision)
                .await;

            enum Outcome {
                Raw(String),
                Done(Option<String>, ResponseType),
            }

            let outcome = if use_terminal {
                let terminal_cancelled = Arc::new(AtomicBool::new(false));
                let mut terminal_input = tokio::task::spawn_blocking({
                    let terminal_cancelled = Arc::clone(&terminal_cancelled);
                    move || Self::read_user_input_with_esc(timeout_duration, terminal_cancelled)
                });
                tokio::select! {
                    result = &mut terminal_input => {
                        match result {
                            Ok(Ok(Some(text))) => Outcome::Raw(text),
                            Ok(Ok(None)) => {
                                println!("\nDecision skipped");
                                completer.complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                }).await;
                                Outcome::Done(None, ResponseType::Cancelled)
                            }
                            _ => {
                                completer.complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Timeout,
                                }).await;
                                Outcome::Done(None, ResponseType::Timeout)
                            }
                        }
                    }
                    result = PendingPromptRegistry::recv_maybe_timeout(rx, timeout_duration) => {
                        Self::stop_terminal_prompt(&terminal_cancelled, &mut terminal_input).await;
                        match result {
                            Ok(MessageContent::Response { answer: Some(a), response_type: ResponseType::Text }) => {
                                Outcome::Raw(a)
                            }
                            Ok(MessageContent::Response { response_type, .. }) => {
                                Outcome::Done(None, response_type)
                            }
                            _ => {
                                completer.complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Timeout,
                                }).await;
                                Outcome::Done(None, ResponseType::Timeout)
                            }
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        Self::stop_terminal_prompt(&terminal_cancelled, &mut terminal_input).await;
                        println!("\n Cancelled");
                        completer.complete(MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::Cancelled,
                        }).await;
                        Outcome::Done(None, ResponseType::Cancelled)
                    }
                }
            } else {
                tokio::select! {
                    result = PendingPromptRegistry::recv_maybe_timeout(rx, timeout_duration) => {
                        match result {
                            Ok(MessageContent::Response { answer: Some(a), response_type: ResponseType::Text }) => {
                                Outcome::Raw(a)
                            }
                            Ok(MessageContent::Response { response_type, .. }) => {
                                Outcome::Done(None, response_type)
                            }
                            _ => {
                                completer.complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Timeout,
                                }).await;
                                Outcome::Done(None, ResponseType::Timeout)
                            }
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("\n Cancelled");
                        completer.complete(MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::Cancelled,
                        }).await;
                        Outcome::Done(None, ResponseType::Cancelled)
                    }
                }
            };

            match outcome {
                Outcome::Done(id, rt) => break (id, String::new(), 0, rt),
                Outcome::Raw(raw) => {
                    if let Some((oid, lbl, idx)) = Self::resolve_decision_answer(&raw, &options) {
                        completer
                            .complete(MessageContent::Response {
                                answer: Some(oid.clone()),
                                response_type: ResponseType::Text,
                            })
                            .await;
                        break (Some(oid), lbl, idx, ResponseType::Text);
                    } else {
                        tracing::warn!(
                            "DECISION_UNKNOWN_ANSWER: '{}' does not match any option id, label, or index",
                            raw
                        );
                        println!(
                            "\nDECISION_UNKNOWN_ANSWER: '{}' does not match any option. Try again.",
                            raw
                        );
                        if use_terminal {
                            print!("Enter option id, label, or number (ESC to skip): ");
                            let _ = io::stdout().flush();
                        }
                        // completer dropped here; loop re-registers fresh rx/completer
                        continue;
                    }
                }
            }
        };

        let mut response_metadata = serde_json::Map::new();
        if let Some(ref oid) = resolved_id {
            response_metadata.insert(
                "option_id".to_string(),
                serde_json::Value::String(oid.clone()),
            );
            response_metadata.insert(
                "label".to_string(),
                serde_json::Value::String(resolved_label.clone()),
            );
            response_metadata.insert(
                "index".to_string(),
                serde_json::Value::Number(serde_json::Number::from(resolved_index)),
            );
        }

        let response_content = MessageContent::Response {
            answer: resolved_id.clone(),
            response_type: response_type.clone(),
        };

        let mut response_message =
            Message::response(message.channel.clone(), response_content, message.id);

        if !response_metadata.is_empty() {
            response_message.metadata = Some(serde_json::Value::Object(response_metadata));
        }

        broadcast_manager.broadcast_message(&response_message).await;

        if let Some(text) = &resolved_id {
            println!("\nDecision resolved: {}", text);
        } else {
            println!("\nDecision response: {:?}", response_type);
        }
        println!();

        response_type
    }

    /// Handle an authorization message. First response (terminal or provider) wins.
    async fn handle_authorization(
        message: Message,
        action: String,
        timeout_secs: u32,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
        config: Option<&Configuration>,
    ) -> ResponseType {
        let use_terminal = io::stdin().is_terminal() && io::stdout().is_terminal();

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Authorization Request [{}]: {}", message.channel, action);
        if timeout_secs > 0 {
            println!("Timeout: {} seconds", timeout_secs);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        if use_terminal {
            print!("Authorize? (Y=yes, n/Enter=no, ESC=skip): ");
            let _ = io::stdout().flush();
        }

        let reply_to_id = broadcast_manager
            .send_to_notification_sinks_and_get_reply_to_id(&message)
            .await;

        let (rx, completer) = pending_registry
            .register(message.id, reply_to_id, PromptType::Authorization)
            .await;
        let timeout_duration = resolve_effective_timeout(timeout_secs, config);

        let decision = if use_terminal {
            let terminal_cancelled = Arc::new(AtomicBool::new(false));
            let mut terminal_input = tokio::task::spawn_blocking({
                let terminal_cancelled = Arc::clone(&terminal_cancelled);
                move || Self::read_authorization_with_esc(timeout_duration, terminal_cancelled)
            });
            tokio::select! {
                result = &mut terminal_input => {
                    match result {
                        Ok(Ok(Some(response_type))) => {
                            let content = MessageContent::Response {
                                answer: None,
                                response_type: response_type.clone(),
                            };
                            completer.complete(content).await;
                            response_type
                        }
                        Ok(Ok(None)) => {
                            println!("\nAuthorization skipped");
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                })
                                .await;
                            ResponseType::Cancelled
                        }
                        Ok(Err(_)) => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::AuthorizationDenied,
                                })
                                .await;
                            ResponseType::AuthorizationDenied
                        }
                        Err(_) => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::AuthorizationDenied,
                                })
                                .await;
                            ResponseType::AuthorizationDenied
                        }
                    }
                }
                result = PendingPromptRegistry::recv_maybe_timeout(rx, timeout_duration) => {
                    Self::stop_terminal_prompt(&terminal_cancelled, &mut terminal_input).await;
                    match result {
                        Ok(MessageContent::Response { response_type, .. }) => response_type,
                        _ => {
                            println!("\nTimeout - DENIED");
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::AuthorizationDenied,
                                })
                                .await;
                            ResponseType::AuthorizationDenied
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    Self::stop_terminal_prompt(&terminal_cancelled, &mut terminal_input).await;
                    println!("\nCancelled - DENIED");
                    completer
                        .complete(MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::AuthorizationDenied,
                        })
                        .await;
                    ResponseType::AuthorizationDenied
                }
            }
        } else {
            tokio::select! {
                result = PendingPromptRegistry::recv_maybe_timeout(rx, timeout_duration) => {
                    match result {
                        Ok(MessageContent::Response { response_type, .. }) => response_type,
                        _ => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::AuthorizationDenied,
                                })
                                .await;
                            ResponseType::AuthorizationDenied
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    completer
                        .complete(MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::AuthorizationDenied,
                        })
                        .await;
                    ResponseType::AuthorizationDenied
                }
            }
        };

        let response_content = MessageContent::Response {
            answer: None,
            response_type: decision.clone(),
        };

        let response_message =
            Message::response(message.channel.clone(), response_content, message.id);

        broadcast_manager.broadcast_message(&response_message).await;

        match decision {
            ResponseType::AuthorizationApproved => {
                println!("\nAuthorization GRANTED");
            }
            ResponseType::AuthorizationDenied => {
                println!("\nAuthorization DENIED");
            }
            ResponseType::Cancelled => {
                println!("\nAuthorization CANCELLED");
            }
            _ => {
                println!("\nAuthorization response: {:?}", decision);
            }
        }
        println!();

        decision
    }

    /// Handle a notification message
    fn handle_notification(text: String, _priority: ailoop_core::models::NotificationPriority) {
        println!("\n {}", text);
    }

    /// Handle a navigate message. First response (terminal or provider) wins.
    async fn handle_navigate(
        message: Message,
        url: String,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
        config: Option<&Configuration>,
    ) -> ResponseType {
        let use_terminal = io::stdin().is_terminal() && io::stdout().is_terminal();

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Navigation Request [{}]: {}", message.channel, url);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        if use_terminal {
            print!("Open in browser? (Y=yes, n/Enter=no, ESC=skip): ");
            let _ = io::stdout().flush();
        }

        let reply_to_id = broadcast_manager
            .send_to_notification_sinks_and_get_reply_to_id(&message)
            .await;

        let (rx, completer) = pending_registry
            .register(message.id, reply_to_id, PromptType::Navigation)
            .await;
        let timeout_duration = resolve_effective_timeout(0, config);

        let decision = if use_terminal {
            let terminal_cancelled = Arc::new(AtomicBool::new(false));
            let mut terminal_input = tokio::task::spawn_blocking({
                let terminal_cancelled = Arc::clone(&terminal_cancelled);
                move || Self::read_authorization_with_esc(timeout_duration, terminal_cancelled)
            });
            tokio::select! {
                result = &mut terminal_input => {
                    match result {
                        Ok(Ok(Some(response_type))) => {
                            let content = MessageContent::Response {
                                answer: None,
                                response_type: response_type.clone(),
                            };
                            completer.complete(content).await;
                            response_type
                        }
                        Ok(Ok(None)) => {
                            println!("\nNavigation skipped");
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                })
                                .await;
                            ResponseType::Cancelled
                        }
                        Ok(Err(_)) => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                })
                                .await;
                            ResponseType::Cancelled
                        }
                        Err(_) => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                })
                                .await;
                            ResponseType::Cancelled
                        }
                    }
                }
                result = PendingPromptRegistry::recv_maybe_timeout(rx, timeout_duration) => {
                    Self::stop_terminal_prompt(&terminal_cancelled, &mut terminal_input).await;
                    match result {
                        Ok(MessageContent::Response { response_type, .. }) => response_type,
                        _ => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                })
                                .await;
                            ResponseType::Cancelled
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    Self::stop_terminal_prompt(&terminal_cancelled, &mut terminal_input).await;
                    println!("\n Cancelled - DENIED");
                    completer
                        .complete(MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::Cancelled,
                        })
                        .await;
                    ResponseType::Cancelled
                }
            }
        } else {
            tokio::select! {
                result = PendingPromptRegistry::recv_maybe_timeout(rx, timeout_duration) => {
                    match result {
                        Ok(MessageContent::Response { response_type, .. }) => response_type,
                        _ => {
                            completer
                                .complete(MessageContent::Response {
                                    answer: None,
                                    response_type: ResponseType::Cancelled,
                                })
                                .await;
                            ResponseType::Cancelled
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nCancelled - DENIED");
                    completer
                        .complete(MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::Cancelled,
                        })
                        .await;
                    ResponseType::Cancelled
                }
            }
        };

        let response_message = Message::response(
            message.channel.clone(),
            MessageContent::Response {
                answer: None,
                response_type: decision.clone(),
            },
            message.id,
        );
        broadcast_manager.broadcast_message(&response_message).await;

        if matches!(decision, ResponseType::AuthorizationApproved) {
            println!("\nOpening browser...");

            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &url])
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&url).spawn();
            }
        } else {
            println!("\nBrowser not opened");
        }
        println!();

        decision
    }

    async fn stop_terminal_prompt<T>(
        cancelled: &Arc<AtomicBool>,
        handle: &mut tokio::task::JoinHandle<Result<Option<T>>>,
    ) {
        cancelled.store(true, Ordering::Relaxed);
        let _ = handle.await;
    }

    /// Read user input with ESC support (ESC to skip, Enter to submit)
    /// Returns Ok(Some(text)) if Enter pressed, Ok(None) if ESC pressed.
    /// When timeout is None, waits indefinitely (no countdown rendered).
    fn read_user_input_with_esc(
        timeout: Option<Duration>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Option<String>> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        let _guard = RawModeGuard;

        let mut buffer = String::new();
        let mut countdown: Option<CountdownRenderer> = timeout.map(CountdownRenderer::new);
        let mut countdown_enabled = true;

        println!("\x1B[s");
        io::stdout().flush()?;

        loop {
            if cancelled.load(Ordering::Relaxed) {
                print!("\r\x1B[2K\x1B[u");
                io::stdout().flush().ok();
                println!();
                return Ok(None);
            }

            if let Some(cd) = &mut countdown {
                if cd.remaining_secs() == 0 {
                    print!("{}", cd.render_final());
                    io::stdout().flush().ok();
                    return Err(anyhow::anyhow!("Question timed out"));
                }
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.kind == KeyEventKind::Press {
                        match key_event.code {
                            KeyCode::Esc => {
                                print!("\r\x1B[2K\x1B[u");
                                io::stdout().flush().ok();
                                println!();
                                return Ok(None);
                            }
                            KeyCode::Enter => {
                                print!("\r\x1B[2K\x1B[u");
                                io::stdout().flush().ok();
                                println!();
                                let answer = buffer.trim().to_string();
                                return Ok(Some(answer));
                            }
                            KeyCode::Char(c) => {
                                buffer.push(c);
                                print!("\x1B[u{}\x1B[s\x1B[B\r", c);
                                io::stdout().flush()?;
                            }
                            KeyCode::Backspace if !buffer.is_empty() => {
                                buffer.pop();
                                print!("\x1B[u\x08 \x08\x1B[s\x1B[B\r");
                                io::stdout().flush()?;
                            }
                            _ => {}
                        }
                    }
                }
            } else if countdown_enabled {
                if let Some(cd) = &mut countdown {
                    if let Some(update) = cd.render_update() {
                        let mut stdout = io::stdout();
                        if stdout.write_all(update.as_bytes()).is_ok() {
                            let _ = stdout.flush();
                        } else {
                            countdown_enabled = false;
                        }
                    }
                }
            }
        }
    }

    /// Read authorization response with ESC support (ESC to skip, Enter to submit)
    /// Returns Ok(Some(ResponseType)) if Enter pressed, Ok(None) if ESC pressed.
    /// When timeout is None, waits indefinitely (no countdown rendered).
    fn read_authorization_with_esc(
        timeout: Option<Duration>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Option<ResponseType>> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        let _guard = RawModeGuard;

        let mut buffer = String::new();
        let mut countdown: Option<CountdownRenderer> = timeout.map(CountdownRenderer::new);
        let mut countdown_enabled = true;

        println!("\x1B[s");
        io::stdout().flush()?;

        loop {
            if cancelled.load(Ordering::Relaxed) {
                print!("\r\x1B[2K\x1B[u");
                io::stdout().flush().ok();
                println!();
                return Ok(None);
            }

            if let Some(cd) = &mut countdown {
                if cd.remaining_secs() == 0 {
                    print!("{}", cd.render_final());
                    io::stdout().flush().ok();
                    return Err(anyhow::anyhow!("Authorization timed out"));
                }
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.kind == KeyEventKind::Press {
                        match key_event.code {
                            KeyCode::Esc => {
                                print!("\r\x1B[2K\x1B[u");
                                io::stdout().flush().ok();
                                println!();
                                return Ok(None);
                            }
                            KeyCode::Enter => {
                                print!("\r\x1B[2K\x1B[u");
                                io::stdout().flush().ok();
                                println!();

                                let normalized = buffer.trim().to_lowercase();
                                let decision = match normalized.as_str() {
                                    "y" | "yes" | "authorized" | "approve" | "ok" => {
                                        ResponseType::AuthorizationApproved
                                    }
                                    "n" | "no" | "denied" | "deny" | "reject" | "" => {
                                        ResponseType::AuthorizationDenied
                                    }
                                    _ => {
                                        eprintln!(
                                            "Invalid input '{}'. Expected Y/n. Defaulting to DENIED.",
                                            buffer.trim()
                                        );
                                        ResponseType::AuthorizationDenied
                                    }
                                };
                                return Ok(Some(decision));
                            }
                            KeyCode::Char(c) => {
                                buffer.push(c);
                                print!("\x1B[u{}\x1B[s\x1B[B\r", c);
                                io::stdout().flush()?;
                            }
                            KeyCode::Backspace if !buffer.is_empty() => {
                                buffer.pop();
                                print!("\x1B[u\x08 \x08\x1B[s\x1B[B\r");
                                io::stdout().flush()?;
                            }
                            _ => {}
                        }
                    }
                }
            } else if countdown_enabled {
                if let Some(cd) = &mut countdown {
                    if let Some(update) = cd.render_update() {
                        let mut stdout = io::stdout();
                        if stdout.write_all(update.as_bytes()).is_ok() {
                            let _ = stdout.flush();
                        } else {
                            countdown_enabled = false;
                        }
                    }
                }
            }
        }
    }
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        disable_raw_mode().ok();
    }
}

/// Axum handler: root GET — WebSocket upgrade or web UI fallback.
async fn root_handler(
    State(state): State<AppState>,
    ws: Option<WebSocketUpgrade>,
) -> impl IntoResponse {
    if let Some(upgrade) = ws {
        let channel_manager = Arc::clone(&state.channel_manager);
        let default_channel = state.default_channel.clone();
        let message_history = Arc::clone(&state.message_history);
        let broadcast_manager = Arc::clone(&state.broadcast_manager);
        upgrade
            .on_upgrade(move |socket| {
                AiloopServer::handle_ws_connection_inner(
                    socket,
                    channel_manager,
                    default_channel,
                    message_history,
                    broadcast_manager,
                )
            })
            .into_response()
    } else if state.web {
        Html(crate::server::web::UI_HTML).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// Global fallback: returns JSON 404 for unmatched paths.
async fn fallback_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({"error": "Not found"})),
    )
}

/// Composes WS upgrade + optional UI + REST into a single Axum Router.
pub fn create_server_router(state: AppState, web: bool) -> axum::Router {
    let state = AppState { web, ..state };

    axum::Router::new()
        .route("/", axum::routing::get(root_handler))
        .merge(crate::server::api::create_api_router())
        .fallback(fallback_handler)
        .with_state(state)
}
