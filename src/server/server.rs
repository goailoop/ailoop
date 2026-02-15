//! Main server integration for ailoop

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

        let api_task = spawn_api_server(
            Arc::clone(&self.message_history),
            Arc::clone(&self.broadcast_manager),
        );

        let message_task = spawn_message_processor(
            Arc::clone(&self.channel_manager),
            Arc::clone(&self.broadcast_manager),
        );

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
            spawn_connection_handler(
                stream,
                addr,
                Arc::clone(&channel_manager),
                self.default_channel.clone(),
                Arc::clone(&self.message_history),
                Arc::clone(&self.broadcast_manager),
            );
        }

        Ok(())
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

                    let response_type = process_message_by_type(
                        message.clone(),
                        &channel_name,
                        Arc::clone(&broadcast_manager),
                    )
                    .await;

                    if matches!(response_type, ResponseType::Cancelled) {
                        channel_manager.enqueue_message(&channel_name, message);
                    }
                }
            }
        }
    }
}

/// Spawn the HTTP API server task
fn spawn_api_server(
    message_history: Arc<crate::server::history::MessageHistory>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
) -> tokio::task::JoinHandle<()> {
    let api_routes = crate::server::api::create_api_routes(message_history, broadcast_manager);

    tokio::spawn(async move {
        warp::serve(api_routes).run(([127, 0, 0, 1], 8081)).await;
    })
}

/// Spawn the message processing task
fn spawn_message_processor(
    channel_manager: Arc<ChannelIsolation>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        AiloopServer::process_queued_messages(channel_manager, broadcast_manager).await;
    })
}

/// Spawn a connection handler task
fn spawn_connection_handler(
    stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    channel_manager: Arc<ChannelIsolation>,
    default_channel: String,
    message_history: Arc<crate::server::history::MessageHistory>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
) {
    tokio::spawn(async move {
        if let Err(e) = handle_connection(
            stream,
            addr,
            channel_manager,
            default_channel,
            message_history,
            broadcast_manager,
        )
        .await
        {
            eprintln!("Connection error: {}", e);
        }
    });
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

    let forward_task = spawn_forward_task(ws_sender, rx);
    handle_incoming_messages(
        &mut ws_receiver,
        addr,
        &mut channel_name,
        Arc::clone(&channel_manager),
        Arc::clone(&message_history),
        Arc::clone(&broadcast_manager),
        connection_id,
    )
    .await;

    forward_task.abort();
    broadcast_manager.remove_viewer(&connection_id).await;
    channel_manager.remove_connection(&channel_name);

    Ok(())
}

/// Spawn task to forward messages to WebSocket
fn spawn_forward_task(
    mut ws_sender: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    rx: tokio::sync::mpsc::UnboundedReceiver<tokio_tungstenite::tungstenite::Message>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(msg) = rx.recv().await {
            if SinkExt::send(&mut ws_sender, msg).await.is_err() {
                break;
            }
        }
    })
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
    channel_manager: Arc<ChannelIsolation>,
    message_history: Arc<crate::server::history::MessageHistory>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    connection_id: String,
) {
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                handle_text_message(
                    &text,
                    addr,
                    channel_name,
                    Arc::clone(&channel_manager),
                    Arc::clone(&message_history),
                    Arc::clone(&broadcast_manager),
                    connection_id.clone(),
                )
                .await;
            }
            Ok(WsMessage::Close(_)) => {
                break;
            }
            Err(e) => {
                eprintln!("[{}] WebSocket error: {}", addr, e);
                break;
            }
            _ => {}
        }
    }
}

/// Handle a text message from WebSocket
async fn handle_text_message(
    text: &str,
    addr: std::net::SocketAddr,
    channel_name: &mut String,
    channel_manager: Arc<ChannelIsolation>,
    message_history: Arc<crate::server::history::MessageHistory>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
    connection_id: String,
) {
    match serde_json::from_str::<Message>(text) {
        Ok(message) => {
            *channel_name = message.channel.clone();

            subscribe_to_channel(&broadcast_manager, &connection_id, &channel_name, addr).await;

            store_and_broadcast_message(&message_history, &broadcast_manager, &message).await;

            channel_manager.enqueue_message(channel_name, message);
        }
        Err(e) => {
            eprintln!("[{}] Failed to parse message: {}", addr, e);
        }
    }
}

/// Subscribe connection to channel
async fn subscribe_to_channel(
    broadcast_manager: &Arc<crate::server::broadcast::BroadcastManager>,
    connection_id: &str,
    channel_name: &str,
    addr: std::net::SocketAddr,
) {
    if let Err(e) = broadcast_manager
        .subscribe_to_channel(connection_id, channel_name)
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
) {
    let channel_name = message.channel.clone();
    let message_clone = message.clone();

    tokio::spawn(async move {
        message_history
            .add_message(&channel_name, message_clone.clone())
            .await;
        broadcast_manager.broadcast_message(&message_clone).await;
    });
}

/// Process message based on its type
async fn process_message_by_type(
    message: Message,
    channel_name: &str,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
) -> ResponseType {
    match &message.content {
        MessageContent::Question {
            text,
            timeout_seconds,
            choices,
        } => {
            handle_question(
                message.clone(),
                text.clone(),
                *timeout_seconds,
                choices.clone(),
                broadcast_manager,
            )
            .await
        }
        MessageContent::Authorization {
            action,
            timeout_seconds,
            ..
        } => {
            handle_authorization(
                message.clone(),
                action.clone(),
                *timeout_seconds,
                broadcast_manager,
            )
            .await
        }
        MessageContent::Notification { text, priority } => {
            handle_notification(text.clone(), priority.clone());
            ResponseType::Text
        }
        MessageContent::Navigate { url } => {
            handle_navigate(message.clone(), url.clone(), broadcast_manager).await
        }
        _ => ResponseType::Text,
    }
}

/// Handle a question message
async fn handle_question(
    message: Message,
    question_text: String,
    timeout_secs: u32,
    choices: Option<Vec<String>>,
    broadcast_manager: Arc<crate::server::broadcast::BroadcastManager>,
) -> ResponseType {
    display_question_prompt(&message, &question_text, timeout_secs, &choices);

    let (answer_text, response_type, selected_index) =
        collect_question_answer(timeout_secs, &choices).await;

    let response = create_question_response(
        &message,
        answer_text,
        response_type.clone(),
        selected_index,
        &choices,
    );

    broadcast_manager.broadcast_message(&response).await;

    display_question_result(&answer_text, &response_type);

    response_type
}

/// Display question prompt to user
fn display_question_prompt(
    message: &Message,
    question_text: &str,
    timeout_secs: u32,
    choices: &Option<Vec<String>>,
) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Question [{}]: {}", message.channel, question_text);
    if timeout_secs > 0 {
        println!("Timeout: {} seconds", timeout_secs);
    }

    if let Some(choices_list) = choices {
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
    io::stdout().flush().ok();
}

/// Collect answer from user with timeout and ESC support
async fn collect_question_answer(
    timeout_secs: u32,
    choices: &Option<Vec<String>>,
) -> (Option<String>, ResponseType, Option<usize>) {
    if timeout_secs > 0 {
        collect_with_timeout_question(timeout_secs, choices).await
    } else {
        collect_without_timeout_question(choices).await
    }
}

async fn collect_with_timeout_question(
    timeout_secs: u32,
    choices: &Option<Vec<String>>,
) -> (Option<String>, ResponseType, Option<usize>) {
    let timeout_duration = Duration::from_secs(timeout_secs as u64);
    tokio::select! {
        result = read_user_input_with_esc() => {
            match result {
                Ok(Some(text)) => {
                    let (final_answer, index) = process_answer(&text, choices);
                    (Some(final_answer), ResponseType::Text, index)
                }
                Ok(None) => {
                    println!("\nQuestion skipped");
                    (None, ResponseType::Cancelled, None)
                }
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

async fn collect_without_timeout_question(
    choices: &Option<Vec<String>>,
) -> (Option<String>, ResponseType, Option<usize>) {
    tokio::select! {
        result = read_user_input_with_esc() => {
            match result {
                Ok(Some(text)) => {
                    let (final_answer, index) = process_answer(&text, choices);
                    (Some(final_answer), ResponseType::Text, index)
                }
                Ok(None) => {
                    println!("\nQuestion skipped");
                    (None, ResponseType::Cancelled, None)
                }
                Err(_) => (None, ResponseType::Cancelled, None),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nCancelled");
            (None, ResponseType::Cancelled, None)
        }
    }
}

/// Create question response message with metadata
fn create_question_response(
    message: &Message,
    answer_text: Option<String>,
    response_type: ResponseType,
    selected_index: Option<usize>,
    choices: &Option<Vec<String>>,
) -> Message {
    let mut metadata = build_response_metadata(selected_index, choices);

    let content = MessageContent::Response {
        answer: answer_text,
        response_type: response_type.clone(),
    };

    let mut response = Message::response(message.channel.clone(), content, message.id);

    if !metadata.is_empty() {
        response.metadata = Some(serde_json::Value::Object(metadata));
    }

    response
}

/// Build metadata for response
fn build_response_metadata(
    selected_index: Option<usize>,
    choices: &Option<Vec<String>>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut metadata = serde_json::Map::new();

    if let Some(idx) = selected_index {
        metadata.insert(
            "index".to_string(),
            serde_json::Value::Number(serde_json::Number::from(idx)),
        );
        if let Some(choices_list) = choices {
            if let Some(selected_choice) = choices_list.get(idx) {
                metadata.insert(
                    "value".to_string(),
                    serde_json::Value::String(selected_choice.clone()),
                );
            }
        }
    }

    metadata
}

/// Display question result to user
fn display_question_result(answer_text: &Option<String>, response_type: &ResponseType) {
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
    display_authorization_prompt(&message, &action, timeout_secs);

    let decision = collect_authorization_decision(timeout_secs).await;

    let response = create_authorization_response(&message, decision.clone());

    broadcast_manager.broadcast_message(&response).await;

    display_authorization_result(&decision);

    decision
}

/// Display authorization prompt
fn display_authorization_prompt(message: &Message, action: &str, timeout_secs: u32) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Authorization Request [{}]: {}", message.channel, action);
    if timeout_secs > 0 {
        println!("Timeout: {} seconds", timeout_secs);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print!("Authorize? (Y/Enter=yes, n=no, ESC=skip): ");
    io::stdout().flush().ok();
}

/// Collect authorization decision from user
async fn collect_authorization_decision(timeout_secs: u32) -> ResponseType {
    if timeout_secs > 0 {
        collect_auth_with_timeout(timeout_secs).await
    } else {
        collect_auth_without_timeout().await
    }
}

async fn collect_auth_with_timeout(timeout_secs: u32) -> ResponseType {
    let timeout_duration = Duration::from_secs(timeout_secs as u64);
    tokio::select! {
        result = read_authorization_with_esc() => {
            match result {
                Ok(Some(response_type)) => response_type,
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

async fn collect_auth_without_timeout() -> ResponseType {
    tokio::select! {
        result = read_authorization_with_esc() => {
            match result {
                Ok(Some(response_type)) => response_type,
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

/// Create authorization response message
fn create_authorization_response(message: &Message, decision: ResponseType) -> Message {
    let content = MessageContent::Response {
        answer: None,
        response_type: decision.clone(),
    };

    Message::response(message.channel.clone(), content, message.id)
}

/// Display authorization result
fn display_authorization_result(decision: &ResponseType) {
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
    display_navigate_prompt(&message, &url);

    let decision = tokio::select! {
        result = read_authorization_with_esc() => {
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
        open_url_in_browser(&url);
    } else {
        println!("Browser not opened");
    }

    decision
}

/// Display navigation prompt
fn display_navigate_prompt(message: &Message, url: &str) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Navigation Request [{}]: {}", message.channel, url);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print!("Open in browser? (Y/Enter=yes, n=no, ESC=skip): ");
    io::stdout().flush().ok();
}

/// Open URL in browser
fn open_url_in_browser(url: &str) {
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
fn process_answer(input: &str, choices: &Option<Vec<String>>) -> (String, Option<usize>) {
    let trimmed = input.trim();

    if let Some(choices_list) = choices {
        if let Ok(num) = trimmed.parse::<usize>() {
            if num >= 1 && num <= choices_list.len() {
                let index = num - 1;
                let selected = choices_list[index].clone();
                return (selected, Some(index));
            }
        }

        for (idx, choice) in choices_list.iter().enumerate() {
            if choice.trim().eq_ignore_ascii_case(trimmed) {
                return (choice.clone(), Some(idx));
            }
        }
    }

    (trimmed.to_string(), None)
}

/// Read user input with ESC support
async fn read_user_input_with_esc() -> Result<Option<String>> {
    tokio::task::spawn_blocking(|| -> Result<Option<String>> {
        enable_raw_mode().context("Failed to enable raw mode")?;

        let mut buffer = String::new();

        loop {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.kind == KeyEventKind::Press {
                        let result = handle_key_event(key_event.code, &mut buffer);
                        if let Some(value) = result {
                            disable_raw_mode().ok();
                            return Ok(value);
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

/// Handle a key event during input
fn handle_key_event(key_code: KeyCode, buffer: &mut String) -> Option<Option<String>> {
    match key_code {
        KeyCode::Esc => {
            disable_raw_mode().ok();
            Some(None)
        }
        KeyCode::Enter => {
            disable_raw_mode().ok();
            println!();
            let answer = buffer.trim().to_string();
            Some(Some(answer))
        }
        KeyCode::Char(c) => {
            buffer.push(c);
            print!("{}", c);
            io::stdout().flush().ok();
            None
        }
        KeyCode::Backspace => {
            if !buffer.is_empty() {
                buffer.pop();
                print!("\x08 \x08");
                io::stdout().flush().ok();
            }
            None
        }
        _ => None,
    }
}

/// Read authorization response with ESC support
async fn read_authorization_with_esc() -> Result<Option<ResponseType>> {
    let result = tokio::task::spawn_blocking(|| -> Result<Option<ResponseType>> {
        enable_raw_mode().context("Failed to enable raw mode")?;

        let mut buffer = String::new();

        loop {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.kind == KeyEventKind::Press {
                        let result = handle_auth_key_event(key_event.code, &mut buffer);
                        if let Some(value) = result {
                            disable_raw_mode().ok();
                            return Ok(value);
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

/// Handle a key event during authorization input
fn handle_auth_key_event(key_code: KeyCode, buffer: &mut String) -> Option<Option<ResponseType>> {
    match key_code {
        KeyCode::Esc => {
            disable_raw_mode().ok();
            Some(None)
        }
        KeyCode::Enter => {
            disable_raw_mode().ok();
            println!();

            let normalized = buffer.trim().to_lowercase();
            let decision = match normalized.as_str() {
                "y" | "yes" | "authorized" | "approve" | "ok" | "" => {
                    ResponseType::AuthorizationApproved
                }
                "n" | "no" | "denied" | "deny" | "reject" => ResponseType::AuthorizationDenied,
                _ => {
                    eprintln!(
                        "Invalid input '{}'. Expected Y/n. Defaulting to APPROVED.",
                        buffer.trim()
                    );
                    ResponseType::AuthorizationApproved
                }
            };
            Some(Some(decision))
        }
        KeyCode::Char(c) => {
            buffer.push(c);
            print!("{}", c);
            io::stdout().flush().ok();
            None
        }
        KeyCode::Backspace => {
            if !buffer.is_empty() {
                buffer.pop();
                print!("\x08 \x08");
                io::stdout().flush().ok();
            }
            None
        }
        _ => None,
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
