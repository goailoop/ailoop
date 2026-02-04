//! Main server integration for ailoop

use crate::channel::ChannelIsolation;
use crate::models::{Configuration, Message, MessageContent, ResponseType};
use crate::server::providers::{PendingPromptRegistry, PromptType, ReplySource};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

/// Main ailoop server
pub struct AiloopServer {
    host: String,
    port: u16,
    default_channel: String,
    channel_manager: Arc<ChannelIsolation>,
    message_history: Arc<crate::server::history::MessageHistory>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    task_storage: Arc<crate::server::task_storage::TaskStorage>,
    pending_prompt_registry: Arc<PendingPromptRegistry>,
    config: Option<Configuration>,
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
        let task_storage = Arc::new(crate::server::task_storage::TaskStorage::new());
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
        }
    }

    /// Attach configuration (for provider loading at startup).
    pub fn with_config(mut self, config: Configuration) -> Self {
        self.config = Some(config);
        self
    }

    /// Start the server
    pub async fn start(self) -> Result<()> {
        use std::net::SocketAddr;
        let address: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid server address")?;

        let listener = TcpListener::bind(&address)
            .await
            .context(format!("Failed to bind to {}", address))?;

        println!("🚀 ailoop server starting on {}", address);
        println!("📺 Default channel: {}", self.default_channel);
        println!("Press Ctrl+C to stop the server");

        println!("🚀 Starting server initialization...");

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
                        let sink =
                            Arc::new(crate::server::providers::TelegramSink::new(t.clone(), c));
                        self.broadcast_manager.add_notification_sink(sink).await;
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
                    (None, _) => {
                        tracing::warn!("Telegram provider skipped: token not set");
                    }
                    (_, None) => {
                        tracing::warn!("Telegram provider skipped: chat_id not configured");
                    }
                }
            }
        }

        // Start HTTP API server
        let api_routes = crate::server::api::create_api_routes(
            Arc::clone(&self.message_history),
            Arc::clone(&self.broadcast_manager),
            Arc::clone(&self.task_storage),
        );
        println!("📋 API routes created");

        // Spawn HTTP API server task (use port + 1 for HTTP API)
        let http_port = self.port + 1;
        let http_host = self.host.clone();
        println!("🌐 HTTP API server starting on {}:{}", http_host, http_port);
        let api_task = tokio::spawn(async move {
            println!("🌐 HTTP API task spawned");
            let addr = format!("{}:{}", http_host, http_port);
            match addr.parse::<std::net::SocketAddr>() {
                Ok(socket_addr) => {
                    println!("🌐 HTTP API server attempting to bind to {}", socket_addr);
                    warp::serve(api_routes).run(socket_addr).await;
                    println!("🌐 HTTP API server task completed");
                }
                Err(e) => {
                    eprintln!("❌ Failed to parse HTTP API address {}: {}", addr, e);
                }
            }
        });
        println!("🌐 HTTP API task spawn requested");

        // Spawn message processing task
        let channel_manager_msg = Arc::clone(&self.channel_manager);
        let broadcast_manager_msg = Arc::clone(&self.broadcast_manager);
        let pending_registry = Arc::clone(&self.pending_prompt_registry);
        let message_task = tokio::spawn(async move {
            Self::process_queued_messages(
                channel_manager_msg,
                broadcast_manager_msg,
                pending_registry,
            )
            .await;
        });

        // Main server loop
        let channel_manager_ws = Arc::clone(&self.channel_manager);
        let server_result = tokio::select! {
            result = self.accept_connections(listener, channel_manager_ws) => result,
            _ = tokio::signal::ctrl_c() => {
                println!("\n🛑 Shutting down server...");
                Ok(())
            }
        };

        // Cleanup
        api_task.abort();
        message_task.abort();

        server_result
    }

    /// Accept WebSocket connections
    async fn accept_connections(
        &self,
        listener: TcpListener,
        channel_manager: Arc<ChannelIsolation>,
    ) -> Result<()> {
        while let Ok((stream, addr)) = listener.accept().await {
            let channel_manager_clone = Arc::clone(&channel_manager);
            let default_channel = self.default_channel.clone();

            let message_history_clone = Arc::clone(&self.message_history);
            let broadcast_clone = Arc::clone(&self.broadcast_manager);
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(
                    stream,
                    addr,
                    channel_manager_clone,
                    default_channel,
                    message_history_clone,
                    broadcast_clone,
                )
                .await
                {
                    eprintln!("Connection error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Handle a single WebSocket connection
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        channel_manager: Arc<ChannelIsolation>,
        default_channel: String,
        message_history: Arc<crate::server::history::MessageHistory>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream)
            .await
            .context("WebSocket handshake failed")?;

        // Connection established (logged silently to avoid interfering with prompts)

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut channel_name = default_channel.clone();

        // Determine connection type (default to Agent, can be changed by protocol)
        let connection_type = crate::server::broadcast::ConnectionType::Agent;
        let connection_id = broadcast_manager.add_viewer(connection_type, tx).await;

        // Track connection
        channel_manager.add_connection(&channel_name);

        // Handle incoming messages and forward outgoing messages
        let forward_task = tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if SinkExt::send(&mut ws_sender, msg).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    // Parse incoming message
                    match serde_json::from_str::<Message>(&text) {
                        Ok(message) => {
                            // Update channel if specified
                            channel_name = message.channel.clone();

                            // Subscribe this connection to the channel so it receives responses
                            let broadcast_clone = Arc::clone(&broadcast_manager);
                            let connection_id_clone = connection_id;
                            let channel_clone = channel_name.clone();
                            if let Err(e) = broadcast_clone
                                .subscribe_to_channel(&connection_id_clone, &channel_clone)
                                .await
                            {
                                eprintln!("[{}] Failed to subscribe to channel: {}", addr, e);
                            }

                            // Store message in history
                            let history_clone = Arc::clone(&message_history);
                            let broadcast_clone2 = Arc::clone(&broadcast_manager);
                            let channel_clone2 = channel_name.clone();
                            let message_clone = message.clone();
                            tokio::spawn(async move {
                                history_clone
                                    .add_message(&channel_clone2, message_clone.clone())
                                    .await;
                                // Broadcast to viewers
                                broadcast_clone2.broadcast_message(&message_clone).await;
                            });

                            // Enqueue message
                            channel_manager.enqueue_message(&channel_name, message);

                            // Message queued (logged silently to avoid interfering with prompts)
                        }
                        Err(e) => {
                            eprintln!("[{}] Failed to parse message: {}", addr, e);
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    // Connection closed normally by client
                    break;
                }
                Err(e) => {
                    eprintln!("[{}] WebSocket error: {}", addr, e);
                    break;
                }
                _ => {}
            }
        }

        // Cleanup
        forward_task.abort();
        broadcast_manager.remove_viewer(&connection_id).await;
        channel_manager.remove_connection(&channel_name);

        Ok(())
    }

    /// Process queued messages and display them to users
    async fn process_queued_messages(
        channel_manager: Arc<ChannelIsolation>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
    ) {
        let mut check_interval = interval(Duration::from_millis(100));

        loop {
            check_interval.tick().await;

            let active_channels = channel_manager.get_active_channels();

            for channel_name in active_channels {
                if let Some(message) = channel_manager.dequeue_message(&channel_name) {
                    println!("\n📬 Processing message from queue [{}]", channel_name);

                    // Process message based on type and get response type
                    let response_type = match &message.content {
                        MessageContent::Question {
                            text,
                            timeout_seconds,
                            choices,
                        } => {
                            let question_text = text.clone();
                            let choices_clone = choices.clone();
                            let broadcast_clone = Arc::clone(&broadcast_manager);
                            let registry = Arc::clone(&pending_registry);
                            Self::handle_question(
                                message.clone(),
                                question_text,
                                *timeout_seconds,
                                choices_clone,
                                broadcast_clone,
                                registry,
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

    /// Handle a question message. First response (terminal or provider) wins.
    async fn handle_question(
        message: Message,
        question_text: String,
        timeout_secs: u32,
        choices: Option<Vec<String>>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
    ) -> ResponseType {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("❓ Question [{}]: {}", message.channel, question_text);
        if timeout_secs > 0 {
            println!("⏱️  Timeout: {} seconds", timeout_secs);
        }
        if let Some(choices_list) = &choices {
            println!("\n📋 Choices:");
            for (idx, choice) in choices_list.iter().enumerate() {
                println!("  {}. {}", idx + 1, choice);
            }
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print!("💬 Your answer (ESC to skip): ");
        } else {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print!("💬 Your answer (ESC to skip): ");
        }
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let (rx, completer, default_timeout) = pending_registry
            .register(message.id, None, PromptType::Question)
            .await;
        let timeout_duration = if timeout_secs > 0 {
            Duration::from_secs(timeout_secs as u64)
        } else {
            default_timeout
        };

        let (answer_text, response_type, selected_index) = tokio::select! {
            result = Self::read_user_input_with_esc() => {
                match result {
                    Ok(Some(text)) => {
                        let (final_answer, index) = Self::process_answer(&text, &choices);
                        let content = MessageContent::Response {
                            answer: Some(final_answer.clone()),
                            response_type: ResponseType::Text,
                        };
                        completer.complete(content).await;
                        (Some(final_answer), ResponseType::Text, index)
                    }
                    Ok(None) => {
                        println!("\n⏭️  Question skipped");
                        let content = MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::Cancelled,
                        };
                        completer.complete(content).await;
                        (None, ResponseType::Cancelled, None)
                    }
                    Err(_) => {
                        let content = MessageContent::Response {
                            answer: None,
                            response_type: ResponseType::Timeout,
                        };
                        completer.complete(content).await;
                        (None, ResponseType::Timeout, None)
                    }
                }
            }
            result = PendingPromptRegistry::recv_with_timeout(rx, timeout_duration) => {
                match result {
                    Ok(MessageContent::Response { answer, response_type }) => {
                        let index = answer.as_ref().and_then(|a| {
                            choices.as_ref().and_then(|c| {
                                c.iter().position(|x| x == a).or_else(|| {
                                    a.parse::<usize>().ok().and_then(|n| {
                                        if n >= 1 && n <= c.len() {
                                            Some(n - 1)
                                        } else {
                                            None
                                        }
                                    })
                                })
                            })
                        });
                        (answer, response_type, index)
                    }
                    _ => (None, ResponseType::Timeout, None),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n⚠️  Cancelled");
                let content = MessageContent::Response {
                    answer: None,
                    response_type: ResponseType::Cancelled,
                };
                completer.complete(content).await;
                (None, ResponseType::Cancelled, None)
            }
        };

        // Create response with metadata including index if multiple choice
        let mut response_metadata = serde_json::Map::new();
        if let Some(idx) = selected_index {
            response_metadata.insert(
                "index".to_string(),
                serde_json::Value::Number(serde_json::Number::from(idx)),
            );
            if let Some(choices_list) = &choices {
                if let Some(selected_choice) = choices_list.get(idx) {
                    response_metadata.insert(
                        "value".to_string(),
                        serde_json::Value::String(selected_choice.clone()),
                    );
                }
            }
        }

        let response_content = MessageContent::Response {
            answer: answer_text.clone(),
            response_type: response_type.clone(),
        };

        let mut response_message =
            Message::response(message.channel.clone(), response_content, message.id);

        // Add metadata with index and value if multiple choice
        if !response_metadata.is_empty() {
            response_message.metadata = Some(serde_json::Value::Object(response_metadata));
        }

        // Send response back via broadcast manager
        broadcast_manager.broadcast_message(&response_message).await;

        // Newline so "Response sent" is on its own line when reply came from Telegram
        if let Some(text) = &answer_text {
            if text.is_empty() {
                println!("\n✅ Response sent: (empty answer)");
            } else {
                println!("\n✅ Response sent: {}", text);
            }
        } else {
            println!("\n📤 Response sent: {:?}", response_type);
        }
        println!();

        // Return the response type so caller knows if message was cancelled
        response_type
    }

    /// Handle an authorization message. First response (terminal or provider) wins.
    async fn handle_authorization(
        message: Message,
        action: String,
        timeout_secs: u32,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
    ) -> ResponseType {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🔐 Authorization Request [{}]: {}", message.channel, action);
        if timeout_secs > 0 {
            println!("⏱️  Timeout: {} seconds", timeout_secs);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        print!("💬 Authorize? (Y/Enter=yes, n=no, ESC=skip): ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let (rx, completer, default_timeout) = pending_registry
            .register(message.id, None, PromptType::Authorization)
            .await;
        let timeout_duration = if timeout_secs > 0 {
            Duration::from_secs(timeout_secs as u64)
        } else {
            default_timeout
        };

        let decision = tokio::select! {
            result = Self::read_authorization_with_esc() => {
                match result {
                    Ok(Some(response_type)) => {
                        let content = MessageContent::Response {
                            answer: None,
                            response_type: response_type.clone(),
                        };
                        completer.complete(content).await;
                        response_type
                    }
                    Ok(None) => {
                        println!("\n⏭️  Authorization skipped");
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
                                response_type: ResponseType::AuthorizationDenied,
                            })
                            .await;
                        ResponseType::AuthorizationDenied
                    }
                }
            }
            result = PendingPromptRegistry::recv_with_timeout(rx, timeout_duration) => {
                match result {
                    Ok(MessageContent::Response { response_type, .. }) => response_type,
                    _ => {
                        println!("\n⏱️  Timeout - DENIED");
                        ResponseType::AuthorizationDenied
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n⚠️  Cancelled - DENIED");
                completer
                    .complete(MessageContent::Response {
                        answer: None,
                        response_type: ResponseType::AuthorizationDenied,
                    })
                    .await;
                ResponseType::AuthorizationDenied
            }
        };

        let response_content = MessageContent::Response {
            answer: None,
            response_type: decision.clone(),
        };

        let response_message =
            Message::response(message.channel.clone(), response_content, message.id);

        // Send response back via broadcast manager
        broadcast_manager.broadcast_message(&response_message).await;

        // Newline so result is on its own line when reply came from Telegram
        match decision {
            ResponseType::AuthorizationApproved => {
                println!("\n✅ Authorization GRANTED");
            }
            ResponseType::AuthorizationDenied => {
                println!("\n❌ Authorization DENIED");
            }
            ResponseType::Cancelled => {
                println!("\n⏭️  Authorization CANCELLED");
            }
            _ => {
                println!("\n📤 Authorization response: {:?}", decision);
            }
        }
        println!();

        // Return the decision so caller knows if message was cancelled
        decision
    }

    /// Handle a notification message
    fn handle_notification(text: String, _priority: crate::models::NotificationPriority) {
        println!("\n💬 {}", text);
    }

    /// Handle a navigate message. First response (terminal or provider) wins.
    async fn handle_navigate(
        message: Message,
        url: String,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
        pending_registry: Arc<PendingPromptRegistry>,
    ) -> ResponseType {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🌐 Navigation Request [{}]: {}", message.channel, url);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        print!("💬 Open in browser? (Y/Enter=yes, n=no, ESC=skip): ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let (rx, completer, default_timeout) = pending_registry
            .register(message.id, None, PromptType::Navigation)
            .await;

        let decision = tokio::select! {
            result = Self::read_authorization_with_esc() => {
                match result {
                    Ok(Some(response_type)) => {
                        let content = MessageContent::Response {
                            answer: None,
                            response_type: response_type.clone(),
                        };
                        completer.complete(content).await;
                        response_type
                    }
                    Ok(None) => {
                        println!("\n⏭️  Navigation skipped");
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
            result = PendingPromptRegistry::recv_with_timeout(rx, default_timeout) => {
                match result {
                    Ok(MessageContent::Response { response_type, .. }) => response_type,
                    _ => ResponseType::Cancelled,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n⚠️  Cancelled - DENIED");
                completer
                    .complete(MessageContent::Response {
                        answer: None,
                        response_type: ResponseType::Cancelled,
                    })
                    .await;
                ResponseType::Cancelled
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

        // Newline so result is on its own line when reply came from Telegram
        if matches!(decision, ResponseType::AuthorizationApproved) {
            println!("\n✅ Opening browser...");

            // Try to open URL in browser (platform-specific)
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
            println!("\n⏭️  Browser not opened");
        }
        println!();

        // Return the decision (Cancelled if skipped, so it can be re-enqueued)
        decision
    }

    /// Process answer for multiple choice questions
    /// Returns (answer_text, selected_index)
    fn process_answer(input: &str, choices: &Option<Vec<String>>) -> (String, Option<usize>) {
        let trimmed = input.trim();

        // If multiple choice, try to parse as index
        if let Some(choices_list) = choices {
            // Try to parse as number (1-based index)
            if let Ok(num) = trimmed.parse::<usize>() {
                if num >= 1 && num <= choices_list.len() {
                    let index = num - 1; // Convert to 0-based
                    let selected = choices_list[index].clone();
                    return (selected, Some(index));
                }
            }
            // Try to match by text (case-insensitive)
            for (idx, choice) in choices_list.iter().enumerate() {
                if choice.trim().eq_ignore_ascii_case(trimmed) {
                    return (choice.clone(), Some(idx));
                }
            }
        }

        // Return as-is for text questions or if no match found
        (trimmed.to_string(), None)
    }

    /// Read user input with ESC support (ESC to skip, Enter to submit)
    /// Returns Ok(Some(text)) if Enter pressed, Ok(None) if ESC pressed
    async fn read_user_input_with_esc() -> Result<Option<String>> {
        tokio::task::spawn_blocking(|| -> Result<Option<String>> {
            // Enable raw mode to read characters
            enable_raw_mode().context("Failed to enable raw mode")?;

            let mut buffer = String::new();

            loop {
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(key_event) = event::read()? {
                        if key_event.kind == KeyEventKind::Press {
                            match key_event.code {
                                KeyCode::Esc => {
                                    // ESC pressed - skip question
                                    disable_raw_mode().ok();
                                    return Ok(None);
                                }
                                KeyCode::Enter => {
                                    // Enter pressed - submit answer (even if empty)
                                    disable_raw_mode().ok();
                                    println!(); // New line after Enter
                                                // Always return the buffer content, even if empty
                                                // Empty string is a valid answer
                                    let answer = buffer.trim().to_string();
                                    return Ok(Some(answer));
                                }
                                KeyCode::Char(c) => {
                                    // Regular character
                                    buffer.push(c);
                                    print!("{}", c);
                                    io::stdout().flush()?;
                                }
                                KeyCode::Backspace => {
                                    // Backspace
                                    if !buffer.is_empty() {
                                        buffer.pop();
                                        print!("\x08 \x08"); // Backspace, space, backspace
                                        io::stdout().flush()?;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        })
        .await
        .context("Failed to spawn blocking task")?
        .context("Failed to read input")
    }

    /// Read authorization response with ESC support (ESC to skip, Enter to submit)
    /// Returns Ok(Some(ResponseType)) if Enter pressed, Ok(None) if ESC pressed
    async fn read_authorization_with_esc() -> Result<Option<ResponseType>> {
        let result = tokio::task::spawn_blocking(|| -> Result<Option<ResponseType>> {
            // Enable raw mode to read characters
            enable_raw_mode().context("Failed to enable raw mode")?;

            let mut buffer = String::new();

            loop {
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(key_event) = event::read()? {
                        if key_event.kind == KeyEventKind::Press {
                            match key_event.code {
                                KeyCode::Esc => {
                                    // ESC pressed - skip
                                    disable_raw_mode().ok();
                                    return Ok(None);
                                }
                                KeyCode::Enter => {
                                    // Enter pressed - parse and return decision
                                    disable_raw_mode().ok();
                                    println!(); // New line after Enter

                                    let normalized = buffer.trim().to_lowercase();
                                    let decision = match normalized.as_str() {
                                        "y" | "yes" | "authorized" | "approve" | "ok" | "" => {
                                            // Empty input (just Enter) defaults to approved
                                            ResponseType::AuthorizationApproved
                                        }
                                        "n" | "no" | "denied" | "deny" | "reject" => {
                                            ResponseType::AuthorizationDenied
                                        }
                                        _ => {
                                            // Invalid input - default to approved (safer default)
                                            eprintln!("⚠️  Invalid input '{}'. Expected Y/n. Defaulting to APPROVED.", buffer.trim());
                                            ResponseType::AuthorizationApproved
                                        }
                                    };
                                    return Ok(Some(decision));
                                }
                                KeyCode::Char(c) => {
                                    // Regular character
                                    buffer.push(c);
                                    print!("{}", c);
                                    io::stdout().flush()?;
                                }
                                KeyCode::Backspace => {
                                    // Backspace
                                    if !buffer.is_empty() {
                                        buffer.pop();
                                        print!("\x08 \x08"); // Backspace, space, backspace
                                        io::stdout().flush()?;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        })
        .await
        .context("Failed to spawn blocking task")?
        .context("Failed to read input")?;

        Ok(result)
    }
}
