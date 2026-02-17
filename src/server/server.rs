//! Main server integration for ailoop

mod handlers_types;
use handlers_types::{
    create_response_metadata, parse_authorization_input, process_multiple_choice, AuthContext,
    AuthDecision, InputResult, QuestionContext,
};

use crate::channel::ChannelIsolation;
use crate::models::{Message, MessageContent, ResponseType};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::{interval, timeout};
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

    /// Start the server
    pub async fn start(self) -> Result<()> {
        use std::net::SocketAddr;
        let address: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid server address")?;

        let listener = TcpListener::bind(&address)
            .await
            .context(format!("Failed to bind to {}", address))?;

        println!("ailoop server starting on {}", address);
        println!("Default channel: {}", self.default_channel);
        println!("Press Ctrl+C to stop the server");

        let api_routes = crate::server::api::create_api_routes(
            Arc::clone(&self.message_history),
            Arc::clone(&self.broadcast_manager),
        );

        let api_task = tokio::spawn(async move {
            warp::serve(api_routes).run(([127, 0, 0, 1], 8081)).await;
        });

        let channel_manager_msg = Arc::clone(&self.channel_manager);
        let broadcast_manager_msg = Arc::clone(&self.broadcast_manager);
        let message_task = tokio::spawn(async move {
            Self::process_queued_messages(channel_manager_msg, broadcast_manager_msg).await;
        });

        let channel_manager_ws = Arc::clone(&self.channel_manager);
        let server_result = tokio::select! {
            result = self.accept_connections(listener, channel_manager_ws) => result,
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down server...");
                Ok(())
            }
        };

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

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut channel_name = default_channel.clone();

        let connection_type = crate::server::broadcast::ConnectionType::Agent;
        let connection_id = broadcast_manager.add_viewer(connection_type, tx).await;

        channel_manager.add_connection(&channel_name);

        let forward_task = tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if SinkExt::send(&mut ws_sender, msg).await.is_err() {
                    break;
                }
            }
        });

        Self::handle_incoming_messages(
            &mut ws_receiver,
            addr,
            &mut channel_name,
            connection_id,
            &broadcast_manager,
            &message_history,
            &channel_manager,
        )
        .await?;

        forward_task.abort();
        broadcast_manager.remove_viewer(&connection_id).await;
        channel_manager.remove_connection(&channel_name);

        Ok(())
    }

    /// Handle incoming WebSocket messages
    async fn handle_incoming_messages(
        ws_receiver: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        addr: std::net::SocketAddr,
        channel_name: &mut String,
        connection_id: String,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
        message_history: &Arc<crate::server::history::MessageHistory>,
        channel_manager: &Arc<ChannelIsolation>,
    ) -> Result<()> {
        while let Some(msg) = ws_receiver.next().await {
            let should_break = Self::process_websocket_message(
                msg,
                addr,
                channel_name,
                &connection_id,
                broadcast_manager,
                message_history,
                channel_manager,
            )
            .await?;

            if should_break {
                break;
            }
        }

        Ok(())
    }

    /// Process a single WebSocket message
    async fn process_websocket_message(
        msg: Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>>,
        addr: std::net::SocketAddr,
        channel_name: &mut String,
        connection_id: &str,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
        message_history: &Arc<crate::server::history::MessageHistory>,
        channel_manager: &Arc<ChannelIsolation>,
    ) -> Result<bool> {
        match msg {
            Ok(WsMessage::Text(text)) => {
                Self::handle_text_message(
                    &text,
                    addr,
                    channel_name,
                    connection_id,
                    broadcast_manager,
                    message_history,
                    channel_manager,
                )
                .await;
                Ok(false)
            }
            Ok(WsMessage::Close(_)) => Ok(true),
            Err(e) => {
                eprintln!("[{}] WebSocket error: {}", addr, e);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Handle text message from WebSocket
    async fn handle_text_message(
        text: &str,
        addr: std::net::SocketAddr,
        channel_name: &mut String,
        connection_id: &str,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
        message_history: &Arc<crate::server::history::MessageHistory>,
        channel_manager: &Arc<ChannelIsolation>,
    ) {
        match serde_json::from_str::<Message>(text) {
            Ok(message) => {
                *channel_name = message.channel.clone();

                Self::subscribe_to_channel(broadcast_manager, connection_id, channel_name, addr)
                    .await;

                Self::store_and_broadcast_message(
                    message_history,
                    broadcast_manager,
                    &message,
                    channel_name,
                )
                .await;

                channel_manager.enqueue_message(channel_name, message);
            }
            Err(e) => {
                eprintln!("[{}] Failed to parse message: {}", addr, e);
            }
        }
    }

    /// Subscribe connection to a channel
    async fn subscribe_to_channel(
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
        connection_id: &str,
        channel_name: &str,
        addr: std::net::SocketAddr,
    ) {
        let broadcast_clone = Arc::clone(broadcast_manager);
        let connection_id_clone = connection_id.to_string();
        let channel_clone = channel_name.to_string();

        if let Err(e) = broadcast_clone
            .subscribe_to_channel(&connection_id_clone, &channel_clone)
            .await
        {
            eprintln!("[{}] Failed to subscribe to channel: {}", addr, e);
        }
    }

    /// Store message in history and broadcast to viewers
    async fn store_and_broadcast_message(
        message_history: &Arc<crate::server::history::MessageHistory>,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
        message: &Message,
        channel_name: &str,
    ) {
        let history_clone = Arc::clone(message_history);
        let broadcast_clone = Arc::clone(broadcast_manager);
        let channel_clone = channel_name.to_string();
        let message_clone = message.clone();

        tokio::spawn(async move {
            history_clone
                .add_message(&channel_clone, message_clone.clone())
                .await;
            broadcast_clone.broadcast_message(&message_clone).await;
        });
    }

    /// Process queued messages and display them to users
    async fn process_queued_messages(
        channel_manager: Arc<ChannelIsolation>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    ) {
        let mut check_interval = interval(Duration::from_millis(100));

        loop {
            check_interval.tick().await;

            let active_channels = channel_manager.get_active_channels();

            for channel_name in active_channels {
                if let Some(message) = channel_manager.dequeue_message(&channel_name) {
                    println!("\nProcessing message from queue [{}]", channel_name);

                    let response_type = Self::dispatch_message(&message, &broadcast_manager).await;

                    if matches!(response_type, ResponseType::Cancelled) {
                        channel_manager.enqueue_message(&channel_name, message);
                    }
                }
            }
        }
    }

    /// Dispatch message to appropriate handler
    async fn dispatch_message(
        message: &Message,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        match &message.content {
            MessageContent::Question {
                text,
                timeout_seconds,
                choices,
            } => {
                Self::handle_question_message(
                    message,
                    text,
                    *timeout_seconds,
                    choices,
                    broadcast_manager,
                )
                .await
            }
            MessageContent::Authorization {
                action,
                timeout_seconds,
                ..
            } => {
                Self::handle_authorization_message(
                    message,
                    action,
                    *timeout_seconds,
                    broadcast_manager,
                )
                .await
            }
            MessageContent::Notification { text, priority } => {
                Self::handle_notification(text.clone(), priority.clone());
                ResponseType::Text
            }
            MessageContent::Navigate { url } => {
                Self::handle_navigate_message(message, url, broadcast_manager).await
            }
            _ => ResponseType::Text,
        }
    }

    /// Handle question message
    async fn handle_question_message(
        message: &Message,
        text: &str,
        timeout_seconds: u32,
        choices: &Option<Vec<String>>,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        let broadcast_clone = Arc::clone(broadcast_manager);
        Self::handle_question(
            message.clone(),
            text.to_string(),
            timeout_seconds,
            choices.clone(),
            broadcast_clone,
        )
        .await
    }

    /// Handle authorization message
    async fn handle_authorization_message(
        message: &Message,
        action: &str,
        timeout_seconds: u32,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        let broadcast_clone = Arc::clone(broadcast_manager);
        Self::handle_authorization(
            message.clone(),
            action.to_string(),
            timeout_seconds,
            broadcast_clone,
        )
        .await
    }

    /// Handle navigate message
    async fn handle_navigate_message(
        message: &Message,
        url: &str,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        let broadcast_clone = Arc::clone(broadcast_manager);
        Self::handle_navigate(message.clone(), url.to_string(), broadcast_clone).await
    }

    /// Handle a question message
    async fn handle_question(
        message: Message,
        question_text: String,
        timeout_secs: u32,
        choices: Option<Vec<String>>,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        let context = QuestionContext::new(question_text, timeout_secs, choices);
        print_question_prompt(&message, &context);

        let (answer_text, response_type, selected_index) =
            Self::collect_question_answer(&context).await;

        let response_message = Self::create_question_response(
            &message,
            answer_text.clone(),
            response_type.clone(),
            &context.choices,
            selected_index,
        );

        broadcast_manager.broadcast_message(&response_message).await;

        print_question_response(&answer_text, &response_type);

        response_type
    }

    /// Print question prompt
    fn print_question_prompt(message: &Message, context: &QuestionContext) {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Question [{}]: {}", message.channel, context.question_text);
        if context.timeout_secs > 0 {
            println!("Timeout: {} seconds", context.timeout_secs);
        }

        if let Some(choices_list) = &context.choices {
            println!("\nChoices:");
            for (idx, choice) in choices_list.iter().enumerate() {
                println!("  {}. {}", idx + 1, choice);
            }
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print!("Your answer (ESC to skip): ");
        } else {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print!("Your answer (ESC to skip): ");
        }
        let _ = std::io::stdout().flush();
    }

    /// Collect question answer from user
    async fn collect_question_answer(
        context: &QuestionContext,
    ) -> (Option<String>, ResponseType, Option<usize>) {
        if context.timeout_secs > 0 {
            Self::collect_answer_with_timeout(context).await
        } else {
            Self::collect_answer_without_timeout(context).await
        }
    }

    /// Collect answer with timeout
    async fn collect_answer_with_timeout(
        context: &QuestionContext,
    ) -> (Option<String>, ResponseType, Option<usize>) {
        let timeout_duration = Duration::from_secs(context.timeout_secs as u64);
        tokio::select! {
            result = Self::read_user_input_with_esc() => {
                match result {
                    Ok(InputResult::Answer(text)) => {
                        let (final_answer, index) = process_multiple_choice(&text, &context.choices);
                        (Some(final_answer), ResponseType::Text, index)
                    }
                    Ok(InputResult::Skip) => {
                        println!("\nQuestion skipped");
                        (None, ResponseType::Cancelled, None)
                    }
                    Ok(InputResult::Timeout) => (None, ResponseType::Timeout, None),
                    Ok(InputResult::Cancelled) => (None, ResponseType::Cancelled, None),
                    Err(_) => (None, ResponseType::Timeout, None),
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                println!("\nTimeout");
                (None, ResponseType::Timeout, None)
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nCancelled");
                (None, ResponseType::Cancelled, None)
            }
        }
    }

    /// Collect answer without timeout
    async fn collect_answer_without_timeout(
        context: &QuestionContext,
    ) -> (Option<String>, ResponseType, Option<usize>) {
        tokio::select! {
            result = Self::read_user_input_with_esc() => {
                match result {
                    Ok(InputResult::Answer(text)) => {
                        let (final_answer, index) = process_multiple_choice(&text, &context.choices);
                        (Some(final_answer), ResponseType::Text, index)
                    }
                    Ok(InputResult::Skip) => {
                        println!("\nQuestion skipped");
                        (None, ResponseType::Cancelled, None)
                    }
                    _ => (None, ResponseType::Cancelled, None),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nCancelled");
                (None, ResponseType::Cancelled, None)
            }
        }
    }

    /// Create question response message
    fn create_question_response(
        message: &Message,
        answer_text: Option<String>,
        response_type: ResponseType,
        choices: &Option<Vec<String>>,
        selected_index: Option<usize>,
    ) -> Message {
        let metadata = create_response_metadata(selected_index, choices);

        let response_content = MessageContent::Response {
            answer: answer_text.clone(),
            response_type: response_type.clone(),
        };

        let mut response_message =
            Message::response(message.channel.clone(), response_content, message.id);

        if let Some(meta) = metadata {
            response_message.metadata = Some(meta);
        }

        response_message
    }

    /// Print question response
    fn print_question_response(answer_text: &Option<String>, response_type: &ResponseType) {
        if let Some(text) = answer_text {
            if text.is_empty() {
                println!("Response sent: (empty answer)");
            } else {
                println!("Response sent: {}", text);
            }
        } else {
            println!("Response sent: {:?}", response_type);
        }
    }

    /// Handle an authorization message
    async fn handle_authorization(
        message: Message,
        action: String,
        timeout_secs: u32,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        let context = AuthContext::new(action, timeout_secs);
        print_auth_prompt(&message, &context);

        let decision = Self::collect_auth_decision(&context).await;

        Self::send_auth_response(&message, &decision, &broadcast_manager).await;

        print_auth_result(&decision);

        decision
    }

    /// Print authorization prompt
    fn print_auth_prompt(message: &Message, context: &AuthContext) {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "Authorization Request [{}]: {}",
            message.channel, context.action
        );
        if context.timeout_secs > 0 {
            println!("Timeout: {} seconds", context.timeout_secs);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        print!("Authorize? (Y/Enter=yes, n=no, ESC=skip): ");
        let _ = std::io::stdout().flush();
    }

    /// Collect authorization decision
    async fn collect_auth_decision(context: &AuthContext) -> ResponseType {
        if context.timeout_secs > 0 {
            Self::collect_auth_with_timeout(context).await
        } else {
            Self::collect_auth_without_timeout(context).await
        }
    }

    /// Collect authorization with timeout
    async fn collect_auth_with_timeout(context: &AuthContext) -> ResponseType {
        let timeout_duration = Duration::from_secs(context.timeout_secs as u64);
        tokio::select! {
            result = Self::read_authorization_with_esc() => {
                match result {
                    Ok(Some(decision)) => auth_decision_to_response_type(decision),
                    Ok(None) => {
                        println!("\nAuthorization skipped");
                        ResponseType::Cancelled
                    }
                    Err(_) => ResponseType::AuthorizationDenied,
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                println!("\nTimeout - DENIED");
                ResponseType::AuthorizationDenied
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nCancelled - DENIED");
                ResponseType::AuthorizationDenied
            }
        }
    }

    /// Collect authorization without timeout
    async fn collect_auth_without_timeout(context: &AuthContext) -> ResponseType {
        tokio::select! {
            result = Self::read_authorization_with_esc() => {
                match result {
                    Ok(Some(decision)) => auth_decision_to_response_type(decision),
                    Ok(None) => {
                        println!("\nAuthorization skipped");
                        ResponseType::Cancelled
                    }
                    Err(_) => ResponseType::AuthorizationDenied,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nCancelled - DENIED");
                ResponseType::AuthorizationDenied
            }
        }
    }

    /// Send authorization response
    async fn send_auth_response(
        message: &Message,
        decision: &ResponseType,
        broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
    ) {
        let response_content = MessageContent::Response {
            answer: None,
            response_type: decision.clone(),
        };

        let response_message =
            Message::response(message.channel.clone(), response_content, message.id);

        broadcast_manager.broadcast_message(&response_message).await;
    }

    /// Print authorization result
    fn print_auth_result(decision: &ResponseType) {
        match decision {
            ResponseType::AuthorizationApproved => {
                println!("Authorization GRANTED");
            }
            ResponseType::AuthorizationDenied => {
                println!("Authorization DENIED");
            }
            ResponseType::Cancelled => {
                println!("Authorization CANCELLED");
            }
            _ => {
                println!("Authorization response: {:?}", decision);
            }
        }
    }

    /// Convert AuthDecision to ResponseType
    fn auth_decision_to_response_type(decision: AuthDecision) -> ResponseType {
        match decision {
            AuthDecision::Approved => ResponseType::AuthorizationApproved,
            AuthDecision::Denied => ResponseType::AuthorizationDenied,
            AuthDecision::Skip => ResponseType::Cancelled,
        }
    }

    /// Handle a notification message
    fn handle_notification(text: String, _priority: crate::models::NotificationPriority) {
        println!("\n{}", text);
    }

    /// Handle a navigate message
    async fn handle_navigate(
        message: Message,
        url: String,
        broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    ) -> ResponseType {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Navigation Request [{}]: {}", message.channel, url);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        print!("Open in browser? (Y/Enter=yes, n=no, ESC=skip): ");
        let _ = std::io::stdout().flush();

        let decision = tokio::select! {
            result = Self::read_authorization_with_esc() => {
                match result {
                    Ok(Some(response_type)) => response_type,
                    Ok(None) => {
                        println!("\nNavigation skipped");
                        ResponseType::Cancelled
                    }
                    Err(_) => ResponseType::Cancelled,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nCancelled - DENIED");
                ResponseType::Cancelled
            }
        };

        if matches!(decision, ResponseType::AuthorizationApproved) {
            println!("Opening browser...");
            Self::open_browser_url(&url);
        } else {
            println!("Browser not opened");
        }

        decision
    }

    /// Open URL in browser
    fn open_browser_url(url: &str) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(url).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(url).spawn();
        }
    }

    /// Process answer for multiple choice questions
    /// Read user input with ESC support
    async fn read_user_input_with_esc() -> Result<InputResult> {
        tokio::task::spawn_blocking(|| -> Result<InputResult> {
            enable_raw_mode().context("Failed to enable raw mode")?;

            let mut buffer = String::new();

            loop {
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(key_event) = event::read()? {
                        if key_event.kind == KeyEventKind::Press {
                            match key_event.code {
                                KeyCode::Esc => {
                                    disable_raw_mode().ok();
                                    return Ok(InputResult::Skip);
                                }
                                KeyCode::Enter => {
                                    disable_raw_mode().ok();
                                    println!();
                                    let answer = buffer.trim().to_string();
                                    return Ok(InputResult::Answer(answer));
                                }
                                KeyCode::Char(c) => {
                                    buffer.push(c);
                                    print!("{}", c);
                                    io::stdout().flush()?;
                                }
                                KeyCode::Backspace => {
                                    if !buffer.is_empty() {
                                        buffer.pop();
                                        print!("\x08 \x08");
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

    /// Read authorization response with ESC support
    async fn read_authorization_with_esc() -> Result<Option<AuthDecision>> {
        let result = tokio::task::spawn_blocking(|| -> Result<Option<AuthDecision>> {
            enable_raw_mode().context("Failed to enable raw mode")?;

            let mut buffer = String::new();

            loop {
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(key_event) = event::read()? {
                        if key_event.kind == KeyEventKind::Press {
                            match key_event.code {
                                KeyCode::Esc => {
                                    disable_raw_mode().ok();
                                    return Ok(None);
                                }
                                KeyCode::Enter => {
                                    disable_raw_mode().ok();
                                    println!();

                                    let decision = parse_authorization_input(&buffer);
                                    return Ok(Some(decision));
                                }
                                KeyCode::Char(c) => {
                                    buffer.push(c);
                                    print!("{}", c);
                                    io::stdout().flush()?;
                                }
                                KeyCode::Backspace => {
                                    if !buffer.is_empty() {
                                        buffer.pop();
                                        print!("\x08 \x08");
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
