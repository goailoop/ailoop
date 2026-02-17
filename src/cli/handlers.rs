//! CLI command handlers

mod handlers_types;
use handlers_types::{
    print_error_output, print_output, AuthorizationDecision, CommandParams, JsonResponseBuilder,
    ResponseHandlingResult, UserInputResult,
};

use crate::transport::Transport;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::time::Duration;
use tokio::signal;
use tokio::time::timeout;

/// Handle the 'ask' command
pub async fn handle_ask(
    question: String,
    channel: String,
    timeout_secs: u32,
    server: String,
    json: bool,
) -> Result<()> {
    validate_channel(&channel)?;
    let operation_mode = determine_operation_mode(&server)?;
    let params = CommandParams::new(channel, timeout_secs, server, json);

    if operation_mode.is_server() {
        handle_ask_server_mode(question, params, operation_mode).await
    } else {
        handle_ask_direct_mode(question, params).await
    }
}

/// Validate channel name with error context
fn validate_channel(channel: &str) -> Result<()> {
    crate::channel::validation::validate_channel_name(channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))
}

/// Determine operation mode with error context
fn determine_operation_mode(server: &str) -> Result<crate::mode::OperationMode> {
    crate::mode::determine_operation_mode(Some(server.to_string()))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))
}

/// Handle ask command in server mode
async fn handle_ask_server_mode(
    question: String,
    params: CommandParams,
    operation_mode: crate::mode::OperationMode,
) -> Result<()> {
    let server_url = operation_mode
        .server_url
        .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

    let (question_text, choices) = parse_multiple_choice_question(&question)?;
    let message = create_question_message(
        &params.channel,
        &question_text,
        params.timeout_secs,
        choices.clone(),
    );

    if !params.json {
        print_ask_send_message(&question_text, choices.is_some());
    }

    let response = send_question_to_server(server_url, &params, message).await?;

    handle_ask_response(response, &params).await
}

/// Create a question message
fn create_question_message(
    channel: &str,
    text: &str,
    timeout_secs: u32,
    choices: Option<Vec<String>>,
) -> crate::models::Message {
    crate::models::Message::new(
        channel.to_string(),
        crate::models::SenderType::Agent,
        crate::models::MessageContent::Question {
            text: text.to_string(),
            timeout_seconds: timeout_secs,
            choices,
        },
    )
}

/// Send question to server and get response
async fn send_question_to_server(
    server_url: String,
    params: &CommandParams,
    message: crate::models::Message,
) -> Result<Option<crate::models::Message>> {
    crate::transport::websocket::send_message_and_wait_response(
        server_url,
        params.channel.clone(),
        message,
        params.timeout_secs,
    )
    .await
    .context("Failed to communicate with server")
}

/// Handle ask command in direct mode
async fn handle_ask_direct_mode(question: String, params: CommandParams) -> Result<()> {
    print!("❓ {}: ", question);
    io::stdout().flush().context("Failed to flush stdout")?;

    let response = if params.timeout_secs > 0 {
        collect_input_with_timeout(params.timeout_secs, &params.channel, params.json).await?
    } else {
        collect_input_no_timeout(&params.channel, params.json).await?
    };

    print_response(response.trim(), params.channel, params.json);
    Ok(())
}

/// Parse question for multiple choice format
fn parse_multiple_choice_question(question: &str) -> Result<(String, Option<Vec<String>>)> {
    if question.contains('|') {
        let parts: Vec<&str> = question.split('|').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!(
                "Invalid multiple choice format. Expected: 'question|choice1|choice2|...'"
            ));
        }
        let q_text = parts[0].trim().to_string();
        let choices_vec: Vec<String> = parts[1..].iter().map(|s| s.trim().to_string()).collect();
        Ok((q_text, Some(choices_vec)))
    } else {
        Ok((question.to_string(), None))
    }
}

/// Print ask command sending message
fn print_ask_send_message(question_text: &str, has_choices: bool) {
    if has_choices {
        println!(
            "📤 Sending multiple choice question to server: {}",
            question_text
        );
    } else {
        println!("📤 Sending question to server: {}", question_text);
    }
    println!("⏳ Waiting for response...");
}

/// Handle ask command response from server
async fn handle_ask_response(
    response: Option<crate::models::Message>,
    params: &CommandParams,
) -> Result<()> {
    match response {
        Some(response_msg) => process_response_message(response_msg, params).await,
        None => {
            let json_builder = JsonResponseBuilder::new(params.channel.clone());
            print_error_output(params.json, &json_builder, "timeout", "Question timed out");
            Err(anyhow::anyhow!("Timeout"))
        }
    }
}

/// Process response message from server
async fn process_response_message(
    response_msg: crate::models::Message,
    params: &CommandParams,
) -> Result<()> {
    match &response_msg.content {
        crate::models::MessageContent::Response {
            answer,
            response_type,
        } => process_response_content(
            answer,
            response_type,
            response_msg.metadata.as_ref(),
            params,
        ),
        _ => Err(anyhow::anyhow!("Server sent unexpected message type")),
    }
}

/// Process response content and handle exit codes
fn process_response_content(
    answer: &Option<String>,
    response_type: &crate::models::ResponseType,
    metadata: Option<&serde_json::Value>,
    params: &CommandParams,
) -> Result<()> {
    match response_type {
        crate::models::ResponseType::Text => {
            handle_text_response(answer, metadata, params)?;
            Ok(())
        }
        crate::models::ResponseType::Timeout => {
            handle_ask_timeout(params);
            std::process::exit(1);
        }
        crate::models::ResponseType::Cancelled => {
            handle_ask_cancelled(params);
            std::process::exit(130);
        }
        _ => {
            handle_ask_unknown(response_type, params);
            std::process::exit(1);
        }
    }
}

/// Handle text response
fn handle_text_response(
    answer: &Option<String>,
    metadata: Option<&serde_json::Value>,
    params: &CommandParams,
) -> Result<()> {
    let answer_text = answer.as_deref().unwrap_or("(no answer provided)");
    let json_builder = JsonResponseBuilder::new(params.channel.clone());

    if params.json {
        println!(
            "{}",
            json_builder.response_with_metadata(answer_text, metadata)
        );
    } else {
        print_text_response_plain(answer_text, metadata);
    }
    Ok(())
}

/// Print text response in plain format
fn print_text_response_plain(answer_text: &str, metadata: Option<&serde_json::Value>) {
    if let Some(metadata) = metadata {
        if let (Some(index), Some(value)) = (
            metadata.get("index").and_then(|v| v.as_u64()),
            metadata.get("value").and_then(|v| v.as_str()),
        ) {
            println!(
                "✅ Response received: {} (choice #{}: {})",
                answer_text,
                index + 1,
                value
            );
        } else {
            println!("✅ Response received: {}", answer_text);
        }
    } else {
        println!("✅ Response received: {}", answer_text);
    }
}

/// Handle ask timeout
fn handle_ask_timeout(params: &CommandParams) {
    print_ask_error(params, "timeout", "Question timed out")
}

/// Handle ask cancelled
fn handle_ask_cancelled(params: &CommandParams) {
    print_ask_error(params, "cancelled", "Question was cancelled")
}

/// Handle ask unknown response type
fn handle_ask_unknown(response_type: &crate::models::ResponseType, params: &CommandParams) {
    let msg = format!("Unexpected response type: {:?}", response_type);
    print_ask_error(params, "unknown", &msg)
}

/// Print ask error (consolidated helper)
fn print_ask_error(params: &CommandParams, error_type: &str, message: &str) {
    let json_builder = JsonResponseBuilder::new(params.channel.clone());
    print_error_output(params.json, &json_builder, error_type, message);
}

/// Collect user input with timeout
async fn collect_input_with_timeout(
    timeout_secs: u32,
    channel: &str,
    json: bool,
) -> Result<String> {
    let timeout_duration = Duration::from_secs(timeout_secs as u64);
    tokio::select! {
        result = timeout(timeout_duration, read_user_input()) => {
            match result {
                Ok(Ok(answer)) => Ok(answer),
                Ok(Err(e)) => Err(e),
                Err(_) => {
                    print_timeout_error(timeout_secs, channel, json);
                    std::process::exit(1);
                }
            }
        }
        _ = signal::ctrl_c() => {
            print_cancelled_error(channel, json);
            std::process::exit(130);
        }
    }
}

/// Collect user input without timeout
async fn collect_input_no_timeout(channel: &str, json: bool) -> Result<String> {
    tokio::select! {
        result = read_user_input() => {
            result.context("Failed to read user input")
        }
        _ = signal::ctrl_c() => {
            print_cancelled_error(channel, json);
            std::process::exit(130);
        }
    }
}

/// Print timeout error
fn print_timeout_error(timeout_secs: u32, channel: &str, json: bool) {
    let json_builder = JsonResponseBuilder::new(channel.to_string());
    let msg = format!("Question timed out after {} seconds", timeout_secs);
    print_error_output(json, &json_builder, "timeout", &msg);
}

/// Print cancelled error
fn print_cancelled_error(channel: &str, json: bool) {
    let json_builder = JsonResponseBuilder::new(channel.to_string());
    print_error_output(
        json,
        &json_builder,
        "cancelled",
        "Question cancelled by user (Ctrl+C)",
    );
}

/// Print response
fn print_response(response: &str, channel: String, json: bool) {
    let json_builder = JsonResponseBuilder::new(channel);
    print_output(json, &json_builder, response);
}

/// Read user input from stdin (async wrapper)
async fn read_user_input() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        Ok::<String, io::Error>(buffer)
    })
    .await
    .context("Failed to read user input")?
    .context("Failed to read from stdin")
}

/// Handle the 'authorize' command
pub async fn handle_authorize(
    action: String,
    channel: String,
    timeout_secs: u32,
    server: String,
    json: bool,
) -> Result<()> {
    validate_channel(&channel)?;
    let operation_mode = determine_operation_mode(&server)?;
    let params = CommandParams::new(channel, timeout_secs, server, json);

    if operation_mode.is_server() {
        handle_authorize_server_mode(action, params, operation_mode).await
    } else {
        handle_authorize_direct_mode(action, params).await
    }
}

/// Handle authorize command in server mode
async fn handle_authorize_server_mode(
    action: String,
    params: CommandParams,
    operation_mode: crate::mode::OperationMode,
) -> Result<()> {
    let server_url = operation_mode
        .server_url
        .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

    let message = create_authorization_message(&params.channel, &action, params.timeout_secs);

    if !params.json {
        println!("📤 Sending authorization request to server: {}", action);
        println!("⏳ Waiting for response...");
    }

    let response = send_authorization_to_server(server_url, &params, message).await?;

    handle_authorize_response(response, &params, &action).await
}

/// Create authorization message
fn create_authorization_message(
    channel: &str,
    action: &str,
    timeout_secs: u32,
) -> crate::models::Message {
    crate::models::Message::new(
        channel.to_string(),
        crate::models::SenderType::Agent,
        crate::models::MessageContent::Authorization {
            action: action.to_string(),
            context: None,
            timeout_seconds: timeout_secs,
        },
    )
}

/// Send authorization request to server
async fn send_authorization_to_server(
    server_url: String,
    params: &CommandParams,
    message: crate::models::Message,
) -> Result<Option<crate::models::Message>> {
    crate::transport::websocket::send_message_and_wait_response(
        server_url,
        params.channel.clone(),
        message,
        params.timeout_secs,
    )
    .await
    .context("Failed to communicate with server")
}

/// Handle authorize command in direct mode
async fn handle_authorize_direct_mode(action: String, params: CommandParams) -> Result<()> {
    print_authorization_prompt(&action, &params);
    io::stdout().flush().context("Failed to flush stdout")?;

    let decision = if params.timeout_secs > 0 {
        collect_authorization_with_timeout(
            params.timeout_secs,
            &params.channel,
            params.json,
            &action,
        )
        .await?
    } else {
        collect_authorization_no_timeout(&params.channel, params.json, &action).await?
    };

    let authorized = matches!(decision, AuthorizationDecision::Approved);
    print_authorization_decision(authorized, &params, &action);

    if authorized {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

/// Print authorization prompt
fn print_authorization_prompt(action: &str, params: &CommandParams) {
    println!("🔐 Authorization Request");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Action: {}", action);
    println!("Channel: {}", params.channel);
    if params.timeout_secs > 0 {
        println!("Timeout: {} seconds", params.timeout_secs);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print!("Authorize this action? (authorized/denied): ");
}

/// Collect authorization decision with timeout
async fn collect_authorization_with_timeout(
    timeout_secs: u32,
    channel: &str,
    json: bool,
    _action: &str,
) -> Result<AuthorizationDecision> {
    let timeout_duration = Duration::from_secs(timeout_secs as u64);
    tokio::select! {
        result = timeout(timeout_duration, read_user_input()) => {
            match result {
                Ok(Ok(answer)) => parse_authorization_response(&answer),
                Ok(Err(_)) => Ok(AuthorizationDecision::Denied),
                Err(_) => {
                    print_auth_timeout(channel, json);
                    std::process::exit(1);
                }
            }
        }
        _ = signal::ctrl_c() => {
            print_auth_cancelled(channel, json);
            std::process::exit(1);
        }
    }
}

/// Collect authorization decision without timeout
async fn collect_authorization_no_timeout(
    channel: &str,
    json: bool,
    _action: &str,
) -> Result<AuthorizationDecision> {
    tokio::select! {
        result = read_user_input() => {
            let answer = result.context("Failed to read user input")?;
            parse_authorization_response(&answer)
        }
        _ = signal::ctrl_c() => {
            print_auth_cancelled(channel, json);
            std::process::exit(1);
        }
    }
}

/// Print authorization timeout
fn print_auth_timeout(channel: &str, json: bool) {
    let json_builder = JsonResponseBuilder::new(channel.to_string());
    print_error_output(
        json,
        &json_builder,
        "timeout",
        "No response received. Defaulting to DENIED for security.",
    );
}

/// Print authorization cancelled
fn print_auth_cancelled(channel: &str, json: bool) {
    let json_builder = JsonResponseBuilder::new(channel.to_string());
    print_error_output(
        json,
        &json_builder,
        "cancelled",
        "Cancelled by user (Ctrl+C). Defaulting to DENIED for security.",
    );
}

/// Handle authorize response from server
async fn handle_authorize_response(
    response: Option<crate::models::Message>,
    params: &CommandParams,
    action: &str,
) -> Result<()> {
    match response {
        Some(response_msg) => process_authorization_response(response_msg, params, action),
        None => {
            handle_authorize_timeout(params, action);
            std::process::exit(1);
        }
    }
}

/// Process authorization response from server
fn process_authorization_response(
    response_msg: crate::models::Message,
    params: &CommandParams,
    action: &str,
) -> Result<()> {
    match &response_msg.content {
        crate::models::MessageContent::Response { response_type, .. } => {
            process_auth_response_type(response_type, params, action)
        }
        _ => Err(anyhow::anyhow!("Server sent unexpected message type")),
    }
}

/// Process authorization response type with proper exit codes
fn process_auth_response_type(
    response_type: &crate::models::ResponseType,
    params: &CommandParams,
    action: &str,
) -> Result<()> {
    match response_type {
        crate::models::ResponseType::AuthorizationApproved => {
            handle_auth_approved(params, action);
            Ok(())
        }
        crate::models::ResponseType::AuthorizationDenied => {
            handle_auth_denied(params, action);
            std::process::exit(1);
        }
        crate::models::ResponseType::Timeout => {
            handle_auth_timeout(params, action);
            std::process::exit(1);
        }
        crate::models::ResponseType::Cancelled => {
            handle_auth_cancelled(params, action);
            std::process::exit(130);
        }
        _ => {
            handle_auth_unknown(response_type, params, action);
            std::process::exit(1);
        }
    }
}

/// Handle authorization approved
fn handle_auth_approved(params: &CommandParams, action: &str) {
    print_auth_result(params, action, true, "✅ Authorization GRANTED", None)
}

/// Handle authorization denied
fn handle_auth_denied(params: &CommandParams, action: &str) {
    print_auth_result(params, action, false, "❌ Authorization DENIED", None)
}

/// Handle authorization timeout
fn handle_auth_timeout(params: &CommandParams, action: &str) {
    print_auth_result(
        params,
        action,
        false,
        "⏱️  Timeout: No response received. Defaulting to DENIED for security.",
        Some((
            "timeout",
            "No response received. Defaulting to DENIED for security.",
        )),
    )
}

/// Handle authorization cancelled
fn handle_auth_cancelled(params: &CommandParams, action: &str) {
    print_auth_result(
        params,
        action,
        false,
        "⚠️  Authorization was cancelled (skipped on server)",
        Some((
            "cancelled",
            "Authorization was cancelled (skipped on server)",
        )),
    )
}

/// Handle authorization unknown response
fn handle_auth_unknown(
    response_type: &crate::models::ResponseType,
    params: &CommandParams,
    action: &str,
) {
    let msg = format!("Unexpected response type: {:?}", response_type);
    print_auth_result(params, action, false, &msg, Some(("unknown", &msg)))
}

/// Print authorization result (consolidated helper)
fn print_auth_result(
    params: &CommandParams,
    action: &str,
    authorized: bool,
    plain_msg: &str,
    error_info: Option<(&str, &str)>,
) {
    let json_builder = JsonResponseBuilder::new(params.channel.clone());
    if params.json {
        match error_info {
            Some((error_type, error_msg)) => {
                println!("{}", json_builder.error(error_type, error_msg));
            }
            None => {
                println!("{}", json_builder.authorization(authorized, action));
            }
        }
    } else {
        println!("{}", plain_msg);
    }
}

/// Print authorization decision
fn print_authorization_decision(authorized: bool, params: &CommandParams, action: &str) {
    let json_builder = JsonResponseBuilder::new(params.channel.clone());
    if params.json {
        println!("{}", json_builder.authorization(authorized, action));
    } else if authorized {
        println!("✅ Authorization GRANTED");
    } else {
        println!("❌ Authorization DENIED");
    }
}

/// Parse user input for authorization response
fn parse_authorization_response(input: &str) -> Result<AuthorizationDecision> {
    let normalized = input.trim().to_lowercase();

    match normalized.as_str() {
        "authorized" | "yes" | "y" | "approve" | "ok" => Ok(AuthorizationDecision::Approved),
        "denied" | "no" | "n" | "deny" | "reject" => Ok(AuthorizationDecision::Denied),
        _ => {
            print!("Invalid response. Please enter 'authorized' or 'denied': ");
            io::stdout().flush().context("Failed to flush stdout")?;
            let retry = read_user_input_sync()?;
            parse_authorization_response(&retry)
        }
    }
}

/// Synchronous version for retry logic
fn read_user_input_sync() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .context("Failed to read from stdin")?;
    Ok(buffer)
}

/// Handle the 'say' command
pub async fn handle_say(
    message: String,
    channel: String,
    priority: String,
    _server: String,
) -> Result<()> {
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    let priority_level = validate_priority(&priority);

    let priority_icon = get_priority_icon(priority_level);
    println!(
        "{} [{}] {}",
        priority_icon,
        priority_level.to_uppercase(),
        message
    );
    println!("📺 Channel: {}", channel);

    Ok(())
}

/// Validate priority and return normalized value
fn validate_priority(priority: &str) -> &'static str {
    match priority.to_lowercase().as_str() {
        "low" => "low",
        "normal" => "normal",
        "high" => "high",
        "urgent" => "urgent",
        _ => {
            eprintln!("Warning: Invalid priority '{}', using 'normal'", priority);
            "normal"
        }
    }
}

/// Get icon for priority level
fn get_priority_icon(priority_level: &str) -> &'static str {
    match priority_level {
        "urgent" => "🚨",
        "high" => "⚠️ ",
        "low" => "ℹ️ ",
        _ => "💬",
    }
}

/// Handle the 'serve' command
pub async fn handle_serve(host: String, port: u16, channel: String) -> Result<()> {
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    let server = crate::server::AiloopServer::new(host, port, channel);
    server.start().await
}

/// Handle the 'config' command
pub async fn handle_config_init(config_file: String) -> Result<()> {
    use crate::models::{Configuration, LogLevel};
    use std::path::PathBuf;

    println!("⚙️  Initializing ailoop configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let config_path = resolve_config_path(&config_file)?;

    println!("📄 Config file: {}", config_path.display());

    let mut config = load_or_create_config(&config_path)?;

    run_config_prompts(&mut config)?;

    save_config(&config, &config_path)?;

    println_config_summary(&config);

    Ok(())
}

/// Resolve config file path
fn resolve_config_path(config_file: &str) -> Result<PathBuf> {
    let config_path = if config_file.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        PathBuf::from(config_file.replacen("~/", &format!("{}/", home), 1))
    } else if config_file == "~/.config/ailoop/config.toml" {
        Configuration::default_config_path()
            .map_err(|e| anyhow::anyhow!("Failed to get default config path: {}", e))?
    } else {
        PathBuf::from(config_file)
    };

    Ok(config_path)
}

/// Load existing config or create default
fn load_or_create_config(config_path: &PathBuf) -> Result<Configuration> {
    if config_path.exists() {
        println!("⚠️  Configuration file already exists. Loading existing values...");
        Configuration::load_from_file(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load existing config: {}", e))
    } else {
        println!("✨ Creating new configuration with defaults...");
        Ok(Configuration::default())
    }
}

/// Run interactive config prompts
fn run_config_prompts(config: &mut Configuration) -> Result<()> {
    println!("\n📝 Please answer the following questions (press Enter to use default):\n");

    prompt_timeout_seconds(config)?;
    prompt_default_channel(config)?;
    prompt_log_level(config)?;
    prompt_server_host(config)?;
    prompt_server_port(config)?;

    Ok(())
}

/// Prompt for timeout seconds
fn prompt_timeout_seconds(config: &mut Configuration) -> Result<()> {
    prompt_with_validation(
        format!(
            "Default timeout for questions in seconds [{}]",
            config.timeout_seconds.unwrap_or(0)
        ),
        |input| {
            if let Ok(timeout) = input.trim().parse::<u32>() {
                config.timeout_seconds = Some(timeout);
                Ok(true)
            } else {
                println!("⚠️  Invalid timeout value, using default");
                Ok(false)
            }
        },
    )
}

/// Prompt for default channel
fn prompt_default_channel(config: &mut Configuration) -> Result<()> {
    prompt_with_validation(
        format!("Default channel name [{}]", config.default_channel),
        |input| {
            let channel = input.trim().to_string();
            if crate::channel::validation::validate_channel_name(&channel).is_ok() {
                config.default_channel = channel;
                Ok(true)
            } else {
                println!("⚠️  Invalid channel name, using default");
                Ok(false)
            }
        },
    )
}

/// Prompt for log level
fn prompt_log_level(config: &mut Configuration) -> Result<()> {
    use crate::models::LogLevel;
    let current_level_str = match config.log_level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
    };

    prompt_with_validation(
        format!(
            "Log level (error/warn/info/debug/trace) [{}]",
            current_level_str
        ),
        |input| {
            if let Ok(level) = parse_log_level(input.trim()) {
                config.log_level = level;
                Ok(true)
            } else {
                println!("⚠️  Invalid log level, using default");
                Ok(false)
            }
        },
    )
}

/// Parse log level from string
fn parse_log_level(input: &str) -> Result<crate::models::LogLevel> {
    match input.to_lowercase().as_str() {
        "error" => Ok(crate::models::LogLevel::Error),
        "warn" => Ok(crate::models::LogLevel::Warn),
        "info" => Ok(crate::models::LogLevel::Info),
        "debug" => Ok(crate::models::LogLevel::Debug),
        "trace" => Ok(crate::models::LogLevel::Trace),
        _ => Err(anyhow::anyhow!("Invalid log level")),
    }
}

/// Prompt for server host
fn prompt_server_host(config: &mut Configuration) -> Result<()> {
    prompt_with_validation(
        format!("Server bind address [{}]", config.server_host),
        |input| {
            config.server_host = input.trim().to_string();
            Ok(true)
        },
    )
}

/// Prompt for server port
fn prompt_server_port(config: &mut Configuration) -> Result<()> {
    prompt_with_validation(format!("Server port [{}]", config.server_port), |input| {
        if let Ok(port) = input.trim().parse::<u16>() {
            config.server_port = port;
            Ok(true)
        } else {
            println!("⚠️  Invalid port number, using default");
            Ok(false)
        }
    })
}

/// Generic prompt with validation function
fn prompt_with_validation<F>(prompt: String, mut validate: F) -> Result<()>
where
    F: FnMut(&str) -> Result<bool>,
{
    print!("{}: ", prompt);
    io::stdout().flush()?;
    let input = read_user_input_sync()?;
    if !input.trim().is_empty() {
        validate(&input)?;
    }
    Ok(())
}

/// Save configuration
fn save_config(config: &Configuration, config_path: &PathBuf) -> Result<()> {
    println!("\n🔍 Validating configuration...");
    match config.validate() {
        Ok(()) => {
            println!("✅ Configuration is valid");
        }
        Err(errors) => {
            println!("❌ Configuration validation failed:");
            for error in &errors {
                println!("   - {}", error);
            }
            return Err(anyhow::anyhow!("Configuration validation failed"));
        }
    }

    println!("\n💾 Saving configuration to {}...", config_path.display());
    config
        .save_to_file(config_path)
        .map_err(|e| anyhow::anyhow!("Failed to save configuration: {}", e))?;

    println!("✅ Configuration saved successfully!");
    Ok(())
}

/// Print configuration summary
fn println_config_summary(config: &Configuration) {
    println!("\n📋 Configuration summary:");
    println!(
        "   Default timeout: {} seconds",
        config
            .timeout_seconds
            .map(|t| t.to_string())
            .unwrap_or_else(|| "disabled".to_string())
    );
    println!("   Default channel: {}", config.default_channel);
    println!("   Log level: {:?}", config.log_level);
    println!("   Server: {}:{}", config.server_host, config.server_port);
}

/// Handle the 'image' command
pub async fn handle_image(image_path: String, channel: String, _server: String) -> Result<()> {
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    let is_url = image_path.starts_with("http://") || image_path.starts_with("https://");

    if is_url {
        handle_image_url(&image_path, &channel)?;
    } else {
        handle_image_file(&image_path, &channel)?;
    }

    Ok(())
}

/// Handle image URL
fn handle_image_url(image_path: &str, channel: &str) -> Result<()> {
    println!("🖼️  [{}] Image URL: {}", channel, image_path);
    println!("💡 Please open this URL in your browser to view the image:");
    println!("   {}", image_path);
    Ok(())
}

/// Handle image file
fn handle_image_file(image_path: &str, channel: &str) -> Result<()> {
    let path = std::path::Path::new(image_path);
    if !path.exists() {
        return Err(anyhow::anyhow!("Image file not found: {}", image_path));
    }

    println!("🖼️  [{}] Image file: {}", channel, image_path);
    println!("💡 Image location: {}", path.canonicalize()?.display());

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let img_type = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "JPEG",
            "png" => "PNG",
            "gif" => "GIF",
            "webp" => "WebP",
            "svg" => "SVG",
            _ => "Unknown",
        };
        println!("📋 Image type: {}", img_type);
    }

    println!("💡 Please open this file in an image viewer to view it.");
    Ok(())
}

/// Handle the 'navigate' command
pub async fn handle_navigate(url: String, channel: String, server: String) -> Result<()> {
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow::anyhow!(
            "Invalid URL format. Must start with http:// or https://"
        ));
    }

    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    if operation_mode.is_server() {
        handle_navigate_server_mode(url, channel, operation_mode).await
    } else {
        handle_navigate_direct_mode(url, channel)
    }
}

/// Handle navigate in server mode
async fn handle_navigate_server_mode(
    url: String,
    channel: String,
    operation_mode: crate::mode::OperationMode,
) -> Result<()> {
    let server_url = operation_mode
        .server_url
        .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

    let content = crate::models::MessageContent::Navigate { url: url.clone() };

    let message =
        crate::models::Message::new(channel.clone(), crate::models::SenderType::Agent, content);

    crate::transport::websocket::send_message_no_response(
        server_url.clone(),
        channel.clone(),
        message,
    )
    .await
    .context("Failed to send navigate message to server")?;

    println!("📤 Navigation request sent to server: {}", url);
    Ok(())
}

/// Handle navigate in direct mode
fn handle_navigate_direct_mode(url: String, channel: String) -> Result<()> {
    println!("🧭 [{}] Navigation suggestion", channel);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("URL: {}", url);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 Please navigate to this URL in your browser:");
    println!("   {}", url);

    open_url_in_browser(&url);

    Ok(())
}

/// Open URL in browser (platform-specific)
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

/// Handle the 'forward' command
pub async fn handle_forward(
    channel: String,
    agent_type: Option<String>,
    format: String,
    transport: String,
    url: Option<String>,
    output: Option<String>,
    client_id: Option<String>,
    input: Option<String>,
) -> Result<()> {
    let _ = channel;
    let _ = agent_type;
    let _ = format;
    let _ = transport;
    let _ = url;
    let _ = output;
    let _ = client_id;
    let _ = input;

    Err(anyhow::anyhow!(
        "Forward is supported by the ailoop-cli crate only. Use the `ailoop` binary from \
         ailoop-cli for forward operations."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_ask_placeholder() {
        let result = handle_ask(
            "Test question".to_string(),
            "test-channel".to_string(),
            60,
            "http://localhost:8080".to_string(),
            false,
        )
        .await;

        assert!(result.is_ok());
    }
}
