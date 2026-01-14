//! Main server integration for ailoop

use crate::channel::ChannelIsolation;
use crate::models::{Message, MessageContent, ResponseType};
use crate::server::TerminalUI;
use anyhow::{Result, Context};
use futures_util::StreamExt;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
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

        Self {
            host,
            port,
            default_channel,
            channel_manager,
            message_history,
            broadcast_manager,
        }
    }

    /// Start the server with terminal UI
    pub async fn start(self) -> Result<()> {
        use std::net::SocketAddr;
        let address: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid server address")?;

        let listener = TcpListener::bind(&address).await
            .context(format!("Failed to bind to {}", address))?;

        println!("🚀 ailoop server starting on {}", address);
        println!("📺 Default channel: {}", self.default_channel);
        println!("Press Ctrl+C to stop the server");

        // Create terminal UI with message history
        let message_history_ui = Arc::clone(&self.message_history);
        let terminal = TerminalUI::new(message_history_ui)
            .map_err(|e| anyhow::anyhow!("Failed to initialize terminal UI: {}", e))?;
        let terminal = Arc::new(std::sync::Mutex::new(terminal));

        // Start HTTP API server
        let api_routes = crate::server::api::create_api_routes(
            Arc::clone(&self.message_history),
            Arc::clone(&self.broadcast_manager),
        );

        // Channel for UI updates
        let (ui_tx, mut ui_rx) = mpsc::channel::<ServerStatus>(100);

        // Spawn terminal UI update task
        let channel_manager_clone = Arc::clone(&self.channel_manager);
        let default_channel_clone = self.default_channel.clone();
        tokio::spawn(async move {
            let mut update_interval = interval(Duration::from_millis(500));
            loop {
                tokio::select! {
                    _ = update_interval.tick() => {
                        let status = Self::calculate_status(
                            &channel_manager_clone,
                            &default_channel_clone,
                        );
                        let _ = ui_tx.send(status).await;
                    }
                }
            }
        });

        // Spawn terminal UI render task with channel switching
        // Note: We use spawn_blocking because TerminalUI uses std::sync::Mutex
        // and we need to avoid Send requirements
        let terminal_clone = Arc::clone(&terminal);
        let terminal_task = tokio::task::spawn_blocking(move || {
            // Run terminal UI in a blocking context
            let rt = tokio::runtime::Handle::current();
            let mut render_interval = interval(Duration::from_millis(200));
            
            loop {
                // Check for quit condition
                let should_quit = {
                    if let Ok(mut term) = terminal_clone.lock() {
                        // Use block_on for async operations in blocking context
                        rt.block_on(async {
                            // Handle input (non-blocking check)
                            if let Ok(true) = term.handle_input().await {
                                return true;
                            }
                            false
                        })
                    } else {
                        false
                    }
                };
                
                if should_quit {
                    break;
                }
                
                // Render
                if let Ok(mut term) = terminal_clone.lock() {
                    let _ = rt.block_on(term.render());
                }
                
                // Small sleep to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(200));
            }
        });

        // Spawn HTTP API server task
        let api_task = tokio::spawn(async move {
            warp::serve(api_routes)
                .run(([127, 0, 0, 1], 8081))
                .await;
        });

        // Spawn message processing task
        let channel_manager_msg = Arc::clone(&self.channel_manager);
        let message_task = tokio::spawn(async move {
            Self::process_queued_messages(channel_manager_msg).await;
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
        terminal_task.abort();
        api_task.abort();
        message_task.abort();
        if let Ok(mut term) = terminal.lock() {
            let _ = term.cleanup();
        }

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
                ).await {
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
        let ws_stream = accept_async(stream).await
            .context("WebSocket handshake failed")?;

        println!("[{}] New WebSocket connection", addr);

        let (ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut channel_name = default_channel.clone();

        // Determine connection type (default to Agent, can be changed by protocol)
        let connection_type = crate::server::broadcast::ConnectionType::Agent;
        let connection_id = broadcast_manager.add_viewer(connection_type, tx).await;

        // Track connection
        channel_manager.add_connection(&channel_name);

        // Handle incoming messages and forward outgoing messages
        let mut forward_task = tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
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

                            // Store message in history
                            let history_clone = Arc::clone(&message_history);
                            let broadcast_clone = Arc::clone(&broadcast_manager);
                            let channel_clone = channel_name.clone();
                            let message_clone = message.clone();
                            tokio::spawn(async move {
                                history_clone.add_message(&channel_clone, message_clone.clone()).await;
                                // Broadcast to viewers
                                broadcast_clone.broadcast_message(&message_clone).await;
                            });

                            // Enqueue message
                            channel_manager.enqueue_message(&channel_name, message);

                            println!("[{}] Message queued in channel '{}'", addr, channel_name);
                        }
                        Err(e) => {
                            eprintln!("[{}] Failed to parse message: {}", addr, e);
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("[{}] Connection closed", addr);
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
    async fn process_queued_messages(channel_manager: Arc<ChannelIsolation>) {
        let mut check_interval = interval(Duration::from_millis(100));

        loop {
            check_interval.tick().await;

            let active_channels = channel_manager.get_active_channels();

            for channel_name in active_channels {
                if let Some(message) = channel_manager.dequeue_message(&channel_name) {

                    // Process message based on type
                    match &message.content {
                        MessageContent::Question { text, timeout_seconds } => {
                            // Create a display-friendly version
                            let question_text = text.clone();
                            Self::handle_question(
                                message.clone(),
                                question_text,
                                *timeout_seconds,
                                channel_manager.clone(),
                            ).await;
                        }
                        MessageContent::Authorization { action, timeout_seconds, .. } => {
                            Self::handle_authorization(
                                message.clone(),
                                action.clone(),
                                *timeout_seconds,
                                channel_manager.clone(),
                            ).await;
                        }
                        MessageContent::Notification { text, priority } => {
                            Self::handle_notification(text.clone(), priority.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Handle a question message
    async fn handle_question(
        message: Message,
        question_text: String,
        timeout_secs: u32,
        _channel_manager: Arc<ChannelIsolation>,
    ) {
        print!("\n❓ [{}] {}: ", message.channel, question_text);
        io::stdout().flush().ok();

        let response = if timeout_secs > 0 {
            let timeout_duration = Duration::from_secs(timeout_secs as u64);
            tokio::select! {
                result = Self::read_user_input_async() => {
                    result.unwrap_or_else(|_| {
                        ResponseType::Timeout
                    })
                }
                _ = tokio::time::sleep(timeout_duration) => {
                    println!("\n⏱️  Timeout");
                    ResponseType::Timeout
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n⚠️  Cancelled");
                    ResponseType::Cancelled
                }
            }
        } else {
            tokio::select! {
                result = Self::read_user_input_async() => {
                    result.unwrap_or_else(|_| ResponseType::Cancelled)
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n⚠️  Cancelled");
                    ResponseType::Cancelled
                }
            }
        };

        // Create response message
        let answer = match response {
            ResponseType::Text => {
                // This would be set from the actual input
                None
            }
            _ => None,
        };

        let response_content = MessageContent::Response {
            answer,
            response_type: response,
        };

        let response_message = Message::response(
            message.channel.clone(),
            response_content,
            message.id,
        );

        // TODO: Send response back via WebSocket to the original sender
        // For now, just log it
        println!("📤 Response: {:?}", response_message);
    }

    /// Handle an authorization message
    async fn handle_authorization(
        message: Message,
        action: String,
        timeout_secs: u32,
        _channel_manager: Arc<ChannelIsolation>,
    ) {
        println!("\n🔐 [{}] Authorization Request", message.channel);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Action: {}", action);
        if timeout_secs > 0 {
            println!("Timeout: {} seconds", timeout_secs);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        print!("Authorize? (authorized/denied): ");
        io::stdout().flush().ok();

        let decision = if timeout_secs > 0 {
            let timeout_duration = Duration::from_secs(timeout_secs as u64);
            tokio::select! {
                result = Self::read_authorization_async() => {
                    result.unwrap_or(ResponseType::AuthorizationDenied)
                }
                _ = tokio::time::sleep(timeout_duration) => {
                    println!("\n⏱️  Timeout - DENIED");
                    ResponseType::AuthorizationDenied
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n⚠️  Cancelled - DENIED");
                    ResponseType::AuthorizationDenied
                }
            }
        } else {
            tokio::select! {
                result = Self::read_authorization_async() => {
                    result.unwrap_or(ResponseType::AuthorizationDenied)
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n⚠️  Cancelled - DENIED");
                    ResponseType::AuthorizationDenied
                }
            }
        };

        let response_content = MessageContent::Response {
            answer: None,
            response_type: decision,
        };

        let response_message = Message::response(
            message.channel.clone(),
            response_content,
            message.id,
        );

        println!("📤 Authorization response: {:?}", response_message);
    }

    /// Handle a notification message
    fn handle_notification(text: String, _priority: crate::models::NotificationPriority) {
        println!("\n💬 {}", text);
    }

    /// Read user input asynchronously
    async fn read_user_input_async() -> Result<ResponseType> {
        let _input = tokio::task::spawn_blocking(|| {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            Ok::<String, io::Error>(buffer)
        })
        .await
        .context("Failed to read input")?
        .context("Failed to read from stdin")?;

        Ok(ResponseType::Text) // Simplified for now
    }

    /// Read authorization response asynchronously
    async fn read_authorization_async() -> Result<ResponseType> {
        let input = tokio::task::spawn_blocking(|| {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            Ok::<String, io::Error>(buffer)
        })
        .await
        .context("Failed to read input")?
        .context("Failed to read from stdin")?;

        let normalized = input.trim().to_lowercase();
        match normalized.as_str() {
            "authorized" | "yes" | "y" | "approve" | "ok" => {
                Ok(ResponseType::AuthorizationApproved)
            }
            _ => Ok(ResponseType::AuthorizationDenied),
        }
    }

    /// Calculate current server status
    fn calculate_status(
        channel_manager: &Arc<ChannelIsolation>,
        _default_channel: &str,
    ) -> ServerStatus {
        let active_channels = channel_manager.get_active_channels();
        let mut total_queue = 0;
        let mut total_connections = 0;

        for channel_name in &active_channels {
            total_queue += channel_manager.get_queue_size(channel_name);
            total_connections += channel_manager.get_connection_count(channel_name);
        }

        ServerStatus {
            status: "Running".to_string(),
            total_queue_size: total_queue,
            total_connections,
            active_channels: active_channels.len(),
        }
    }
}
