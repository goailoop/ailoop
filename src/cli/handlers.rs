//! CLI command handlers

use crate::transport::Transport;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::time::Duration;
use tokio::signal;
use tokio::time::timeout;

/// Parameters for question handling
struct QuestionParams {
    text: String,
    timeout_secs: u32,
    choices: Option<Vec<String>>,
}

/// Parameters for timeout handling
struct TimeoutParams {
    timeout_secs: u32,
    channel: String,
    json: bool,
}

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

    if operation_mode.is_server() {
        handle_ask_server(
            &question,
            &channel,
            timeout_secs,
            &server,
            json,
            &operation_mode,
        )
        .await
    } else {
        handle_ask_direct(&question, &channel, timeout_secs, json).await
    }
}

async fn handle_ask_server(
    question: &str,
    channel: &str,
    timeout_secs: u32,
    server: &str,
    json: bool,
    operation_mode: &crate::mode::OperationMode,
) -> Result<()> {
    let server_url = get_server_url(operation_mode)?;
    let question_params = parse_question(question)?;

    let message = create_question_message(channel, &question_params);

    if !json {
        display_sending_message(&question_params.text, question_params.choices.is_some());
    }

    let response = send_and_wait_response(server_url, channel, message, timeout_secs).await?;

    match response {
        Some(msg) => handle_question_response(msg, channel, json).await,
        None => handle_timeout(channel, json, "No response received from server"),
    }
}

async fn handle_ask_direct(
    question: &str,
    channel: &str,
    timeout_secs: u32,
    json: bool,
) -> Result<()> {
    print!("❓ {}: ", question);
    io::stdout().flush().context("Failed to flush stdout")?;

    let timeout_params = TimeoutParams {
        timeout_secs,
        channel: channel.to_string(),
        json,
    };

    let response = collect_user_input_with_timeout(&timeout_params, "Question").await?;

    output_response(&response, channel, json)?;
    Ok(())
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

    if operation_mode.is_server() {
        handle_authorize_server(
            &action,
            &channel,
            timeout_secs,
            &server,
            json,
            &operation_mode,
        )
        .await
    } else {
        handle_authorize_direct(&action, &channel, timeout_secs, json).await
    }
}

async fn handle_authorize_server(
    action: &str,
    channel: &str,
    timeout_secs: u32,
    server: &str,
    json: bool,
    operation_mode: &crate::mode::OperationMode,
) -> Result<()> {
    let server_url = get_server_url(operation_mode)?;
    let message = create_authorization_message(channel, action, timeout_secs);

    if !json {
        println!("📤 Sending authorization request to server: {}", action);
        println!("⏳ Waiting for response...");
    }

    let response = send_and_wait_response(server_url, channel, message, timeout_secs).await?;

    match response {
        Some(msg) => handle_authorization_response(msg, channel, action, json),
        None => handle_authorization_timeout(channel, action, json),
    }
}

async fn handle_authorize_direct(
    action: &str,
    channel: &str,
    timeout_secs: u32,
    json: bool,
) -> Result<()> {
    display_authorization_prompt(action, channel, timeout_secs);

    let timeout_params = TimeoutParams {
        timeout_secs,
        channel: channel.to_string(),
        json,
    };

    let decision = collect_authorization_decision(&timeout_params, action).await?;

    output_authorization_result(decision, action, channel, json)?;

    Ok(())
}

/// Handle the 'say' command
pub async fn handle_say(
    message: String,
    channel: String,
    priority: String,
    _server: String,
) -> Result<()> {
    validate_channel(&channel)?;

    let priority_level = validate_and_normalize_priority(&priority);

    display_notification(&message, &priority_level, &channel);

    Ok(())
}

/// Handle the 'serve' command
pub async fn handle_serve(host: String, port: u16, channel: String) -> Result<()> {
    validate_channel(&channel)?;

    let server = crate::server::AiloopServer::new(host, port, channel);
    server.start().await
}

/// Handle the 'config' command
pub async fn handle_config_init(config_file: String) -> Result<()> {
    use crate::models::{Configuration, LogLevel};
    use std::path::PathBuf;

    println!("⚙️  Initializing ailoop configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let config_path = resolve_config_path(config_file)?;
    println!("📄 Config file: {}", config_path.display());

    let mut config = load_or_create_config(&config_path)?;

    collect_config_values(&mut config)?;

    validate_and_save_config(&config, &config_path)?;

    display_config_summary(&config);

    Ok(())
}

/// Handle the 'image' command
pub async fn handle_image(image_path: String, channel: String, _server: String) -> Result<()> {
    validate_channel(&channel)?;

    let is_url = image_path.starts_with("http://") || image_path.starts_with("https://");

    if is_url {
        display_url_image(&image_path, &channel)?;
    } else {
        display_file_image(&image_path, &channel)?;
    }

    Ok(())
}

/// Handle the 'navigate' command
pub async fn handle_navigate(url: String, channel: String, server: String) -> Result<()> {
    validate_channel(&channel)?;
    validate_url(&url)?;

    let operation_mode = determine_operation_mode(&server)?;

    if operation_mode.is_server() {
        handle_navigate_server(&url, &channel, &operation_mode).await
    } else {
        handle_navigate_direct(&url, &channel)
    }
}

async fn handle_navigate_server(
    url: &str,
    channel: &str,
    operation_mode: &crate::mode::OperationMode,
) -> Result<()> {
    let server_url = get_server_url(operation_mode)?;
    let message = create_navigate_message(channel, url);

    crate::transport::websocket::send_message_no_response(server_url, channel, message)
        .await
        .context("Failed to send navigate message to server")?;

    println!("📤 Navigation request sent to server: {}", url);
    Ok(())
}

fn handle_navigate_direct(url: &str, channel: &str) -> Result<()> {
    println!("🧭 [{}] Navigation suggestion", channel);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("URL: {}", url);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 Please navigate to this URL in your browser:");
    println!("   {}", url);

    open_url_in_browser(url);

    Ok(())
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

// ========== Helper Functions ==========

fn validate_channel(channel: &str) -> Result<()> {
    crate::channel::validation::validate_channel_name(channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))
}

fn determine_operation_mode(server: &str) -> Result<crate::mode::OperationMode> {
    crate::mode::determine_operation_mode(Some(server.to_string()))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))
}

fn get_server_url(operation_mode: &crate::mode::OperationMode) -> Result<String> {
    operation_mode
        .server_url
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))
}

fn parse_question(question: &str) -> Result<QuestionParams> {
    if question.contains('|') {
        parse_multiple_choice_question(question)
    } else {
        Ok(QuestionParams {
            text: question.to_string(),
            timeout_secs: 0,
            choices: None,
        })
    }
}

fn parse_multiple_choice_question(question: &str) -> Result<QuestionParams> {
    let parts: Vec<&str> = question.split('|').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "Invalid multiple choice format. Expected: 'question|choice1|choice2|...'"
        ));
    }
    let text = parts[0].trim().to_string();
    let choices: Vec<String> = parts[1..].iter().map(|s| s.trim().to_string()).collect();

    Ok(QuestionParams {
        text,
        timeout_secs: 0,
        choices: Some(choices),
    })
}

fn create_question_message(channel: &str, params: &QuestionParams) -> crate::models::Message {
    let content = crate::models::MessageContent::Question {
        text: params.text.clone(),
        timeout_seconds: params.timeout_secs,
        choices: params.choices.clone(),
    };

    crate::models::Message::new(
        channel.to_string(),
        crate::models::SenderType::Agent,
        content,
    )
}

fn create_authorization_message(
    channel: &str,
    action: &str,
    timeout_secs: u32,
) -> crate::models::Message {
    let content = crate::models::MessageContent::Authorization {
        action: action.to_string(),
        context: None,
        timeout_seconds: timeout_secs,
    };

    crate::models::Message::new(
        channel.to_string(),
        crate::models::SenderType::Agent,
        content,
    )
}

fn create_navigate_message(channel: &str, url: &str) -> crate::models::Message {
    let content = crate::models::MessageContent::Navigate {
        url: url.to_string(),
    };

    crate::models::Message::new(
        channel.to_string(),
        crate::models::SenderType::Agent,
        content,
    )
}

async fn send_and_wait_response(
    server_url: String,
    channel: &str,
    message: crate::models::Message,
    timeout_secs: u32,
) -> Result<Option<crate::models::Message>> {
    crate::transport::websocket::send_message_and_wait_response(
        server_url,
        channel.to_string(),
        message,
        timeout_secs,
    )
    .await
    .context("Failed to communicate with server")
}

fn display_sending_message(text: &str, is_multiple_choice: bool) {
    if is_multiple_choice {
        println!("📤 Sending multiple choice question to server: {}", text);
    } else {
        println!("📤 Sending question to server: {}", text);
    }
    println!("⏳ Waiting for response...");
}

async fn handle_question_response(
    message: crate::models::Message,
    channel: &str,
    json: bool,
) -> Result<()> {
    if let crate::models::MessageContent::Response {
        answer,
        response_type,
    } = &message.content
    {
        match response_type {
            crate::models::ResponseType::Text => {
                output_text_response(answer, channel, &message, json)?;
                Ok(())
            }
            crate::models::ResponseType::Timeout => {
                output_timeout_error(channel, json, timeout_secs);
                std::process::exit(1);
            }
            crate::models::ResponseType::Cancelled => {
                output_cancelled_error(channel, json);
                std::process::exit(130);
            }
            _ => {
                output_unknown_error(response_type, channel, json);
                std::process::exit(1);
            }
        }
    } else {
        Err(anyhow::anyhow!("Server sent unexpected message type"))
    }
}

fn output_text_response(
    answer: &Option<String>,
    channel: &str,
    message: &crate::models::Message,
    json: bool,
) -> Result<()> {
    let answer_text = answer.as_deref().unwrap_or("(no answer provided)");

    if json {
        let json_response = build_json_response(answer_text, channel, &message.metadata);
        println!("{}", serde_json::to_string_pretty(&json_response)?);
    } else {
        display_text_response(answer_text, channel, &message.metadata);
    }

    Ok(())
}

fn build_json_response(
    answer: &str,
    channel: &str,
    metadata: &Option<serde_json::Value>,
) -> serde_json::Value {
    let mut response = serde_json::json!({
        "response": answer,
        "channel": channel,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Some(meta) = metadata {
        response["metadata"] = meta.clone();
    }

    response
}

fn display_text_response(answer: &str, channel: &str, metadata: &Option<serde_json::Value>) {
    if let Some(meta) = metadata {
        if let (Some(index), Some(value)) = (
            meta.get("index").and_then(|v| v.as_u64()),
            meta.get("value").and_then(|v| v.as_str()),
        ) {
            println!(
                "✅ Response received: {} (choice #{}: {})",
                answer,
                index + 1,
                value
            );
        } else {
            println!("✅ Response received: {}", answer);
        }
    } else {
        println!("✅ Response received: {}", answer);
    }
}

fn output_timeout_error(channel: &str, json: bool, timeout_secs: u32) {
    let message = format!("Question timed out after {} seconds", timeout_secs);
    output_error("timeout", &message, channel, json);
}

fn output_cancelled_error(channel: &str, json: bool) {
    let message = "Question was cancelled".to_string();
    output_error("cancelled", &message, channel, json);
}

fn output_unknown_error(response_type: &crate::models::ResponseType, channel: &str, json: bool) {
    let message = format!("Unexpected response type: {:?}", response_type);
    output_error("unknown", &message, channel, json);
}

fn handle_timeout(channel: &str, json: bool, message: &str) -> Result<()> {
    output_error("timeout", message, channel, json);
    std::process::exit(1);
}

fn output_error(error_type: &str, message: &str, channel: &str, json: bool) {
    if json {
        let json_response = serde_json::json!({
            "error": error_type,
            "message": message,
            "channel": channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string_pretty(&json_response).unwrap());
    } else {
        println!("⏱️  {}: {}", error_type, message);
    }
}

async fn collect_user_input_with_timeout(params: &TimeoutParams, context: &str) -> Result<String> {
    if params.timeout_secs > 0 {
        let timeout_duration = Duration::from_secs(params.timeout_secs as u64);
        tokio::select! {
            result = timeout(timeout_duration, read_user_input()) => {
                match result {
                    Ok(Ok(answer)) => Ok(answer),
                    Ok(Err(e)) => Err(e),
                    Err(_) => {
                        output_timeout_error(&params.channel, params.json, params.timeout_secs);
                        std::process::exit(1);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                output_cancelled_error(&params.channel, params.json);
                std::process::exit(130);
            }
        }
    } else {
        tokio::select! {
            result = read_user_input() => {
                result.context("Failed to read user input")
            }
            _ = signal::ctrl_c() => {
                output_cancelled_error(&params.channel, params.json);
                std::process::exit(130);
            }
        }
    }
}

async fn collect_authorization_decision(
    params: &TimeoutParams,
    action: &str,
) -> Result<AuthorizationDecision> {
    let decision = if params.timeout_secs > 0 {
        collect_with_timeout(params).await?
    } else {
        collect_without_timeout(params).await?
    };

    Ok(decision)
}

async fn collect_with_timeout(params: &TimeoutParams) -> Result<AuthorizationDecision> {
    let timeout_duration = Duration::from_secs(params.timeout_secs as u64);
    tokio::select! {
        result = timeout(timeout_duration, read_user_input()) => {
            match result {
                Ok(Ok(answer)) => parse_authorization_response(&answer),
                Ok(Err(_)) => Ok(AuthorizationDecision::Denied),
                Err(_) => {
                    output_auth_timeout_error(&params.channel, params.json);
                    std::process::exit(1);
                }
            }
        }
        _ = signal::ctrl_c() => {
            output_auth_cancelled_error(&params.channel, params.json);
            std::process::exit(1);
        }
    }
}

async fn collect_without_timeout(params: &TimeoutParams) -> Result<AuthorizationDecision> {
    tokio::select! {
        result = read_user_input() => {
            let answer = result.context("Failed to read user input")?;
            Ok(parse_authorization_response(&answer)?)
        }
        _ = signal::ctrl_c() => {
            output_auth_cancelled_error(&params.channel, params.json);
            std::process::exit(1);
        }
    }
}

fn output_auth_timeout_error(channel: &str, json: bool) {
    let message = "No response received. Defaulting to DENIED for security.";
    let json_response = serde_json::json!({
        "authorized": false,
        "channel": channel,
        "reason": "timeout",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&json_response).unwrap());
    } else {
        println!("⏱️  Timeout: {}", message);
    }
}

fn output_auth_cancelled_error(channel: &str, json: bool) {
    let message = "Cancelled by user (Ctrl+C). Defaulting to DENIED for security.";
    let json_response = serde_json::json!({
        "authorized": false,
        "channel": channel,
        "reason": "cancelled",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if json {
        println!(
            "\n{}",
            serde_json::to_string_pretty(&json_response).unwrap()
        );
    } else {
        println!("\n⚠️  {}", message);
    }
}

async fn handle_authorization_response(
    message: crate::models::Message,
    channel: &str,
    action: &str,
    json: bool,
) -> Result<()> {
    if let crate::models::MessageContent::Response {
        answer: _,
        response_type,
    } = &message.content
    {
        match response_type {
            crate::models::ResponseType::AuthorizationApproved => {
                output_auth_approved(json, action, channel);
                return Ok(());
            }
            crate::models::ResponseType::AuthorizationDenied => {
                output_auth_denied(json, action, channel, "denied");
                std::process::exit(1);
            }
            crate::models::ResponseType::Timeout => {
                output_auth_denied(json, action, channel, "timeout");
                std::process::exit(1);
            }
            crate::models::ResponseType::Cancelled => {
                output_auth_cancelled(json, action, channel);
                std::process::exit(130);
            }
            _ => {
                output_auth_denied(json, action, channel, "unknown");
                std::process::exit(1);
            }
        }
    } else {
        Err(anyhow::anyhow!("Server sent unexpected message type"))
    }
}

fn handle_authorization_timeout(channel: &str, action: &str, json: bool) -> Result<()> {
    output_auth_denied(json, action, channel, "timeout");
    std::process::exit(1);
}

fn output_auth_approved(json: bool, action: &str, channel: &str) {
    if json {
        let json_response = serde_json::json!({
            "authorized": true,
            "action": action,
            "channel": channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string_pretty(&json_response).unwrap());
    } else {
        println!("✅ Authorization GRANTED");
    }
}

fn output_auth_denied(json: bool, action: &str, channel: &str, reason: &str) {
    if json {
        let json_response = serde_json::json!({
            "authorized": false,
            "action": action,
            "channel": channel,
            "reason": reason,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string_pretty(&json_response).unwrap());
    } else if reason == "timeout" {
        println!("⏱️  Timeout: No response received. Defaulting to DENIED for security.");
    } else {
        println!("❌ Authorization DENIED");
    }
}

fn output_auth_cancelled(json: bool, action: &str, channel: &str) {
    if json {
        let json_response = serde_json::json!({
            "authorized": false,
            "action": action,
            "channel": channel,
            "reason": "cancelled",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string_pretty(&json_response).unwrap());
    } else {
        println!("⚠️  Authorization was cancelled (skipped on server)");
    }
}

fn display_authorization_prompt(action: &str, channel: &str, timeout_secs: u32) {
    println!("🔐 Authorization Request");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Action: {}", action);
    println!("Channel: {}", channel);
    if timeout_secs > 0 {
        println!("Timeout: {} seconds", timeout_secs);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print!("Authorize this action? (authorized/denied): ");
    io::stdout().flush().expect("Failed to flush stdout");
}

fn output_authorization_result(
    decision: AuthorizationDecision,
    action: &str,
    channel: &str,
    json: bool,
) -> Result<()> {
    let authorized = matches!(decision, AuthorizationDecision::Approved);

    if json {
        let json_response = serde_json::json!({
            "authorized": authorized,
            "action": action,
            "channel": channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string_pretty(&json_response)?);
    } else if authorized {
        println!("✅ Authorization GRANTED");
    } else {
        println!("❌ Authorization DENIED");
    }

    if authorized {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

fn output_response(response: &str, channel: &str, json: bool) -> Result<()> {
    if json {
        let json_response = serde_json::json!({
            "response": response.trim(),
            "channel": channel,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string_pretty(&json_response)?);
    } else {
        println!("{}", response.trim());
    }

    Ok(())
}

fn validate_and_normalize_priority(priority: &str) -> &str {
    match priority.to_lowercase().as_str() {
        "low" | "normal" | "high" | "urgent" => priority,
        _ => {
            eprintln!("Warning: Invalid priority '{}', using 'normal'", priority);
            "normal"
        }
    }
}

fn display_notification(message: &str, priority: &str, channel: &str) {
    let priority_icon = match priority {
        "urgent" => "🚨",
        "high" => "⚠️ ",
        "low" => "ℹ️ ",
        _ => "💬",
    };

    println!(
        "{} [{}] {}",
        priority_icon,
        priority.to_uppercase(),
        message
    );
    println!("📺 Channel: {}", channel);
}

fn resolve_config_path(config_file: String) -> Result<std::path::PathBuf> {
    use std::path::PathBuf;

    let config_path = if config_file.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        PathBuf::from(config_file.replacen("~/", &format!("{}/", home), 1))
    } else if config_file == "~/.config/ailoop/config.toml" {
        crate::models::Configuration::default_config_path()
            .map_err(|e| anyhow::anyhow!("Failed to get default config path: {}", e))?
    } else {
        PathBuf::from(config_file)
    };

    Ok(config_path)
}

fn load_or_create_config(config_path: &std::path::Path) -> Result<crate::models::Configuration> {
    use crate::models::Configuration;

    let config = if config_path.exists() {
        println!("⚠️  Configuration file already exists. Loading existing values...");
        Configuration::load_from_file(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load existing config: {}", e))?
    } else {
        println!("✨ Creating new configuration with defaults...");
        Configuration::default()
    };

    Ok(config)
}

fn collect_config_values(config: &mut crate::models::Configuration) -> Result<()> {
    use crate::models::LogLevel;

    println!("\n📝 Please answer the following questions (press Enter to use default):\n");

    collect_timeout_value(config)?;
    collect_channel_value(config)?;
    collect_log_level_value(config)?;
    collect_host_value(config)?;
    collect_port_value(config)?;

    Ok(())
}

fn collect_timeout_value(config: &mut crate::models::Configuration) -> Result<()> {
    print!(
        "Default timeout for questions in seconds [{}]: ",
        config.timeout_seconds.unwrap_or(0)
    );
    io::stdout().flush()?;
    let timeout_input = read_user_input_sync()?;
    if !timeout_input.trim().is_empty() {
        if let Ok(timeout) = timeout_input.trim().parse::<u32>() {
            config.timeout_seconds = Some(timeout);
        } else {
            println!("⚠️  Invalid timeout value, using default");
        }
    }
    Ok(())
}

fn collect_channel_value(config: &mut crate::models::Configuration) -> Result<()> {
    print!("Default channel name [{}]: ", config.default_channel);
    io::stdout().flush()?;
    let channel_input = read_user_input_sync()?;
    if !channel_input.trim().is_empty() {
        let channel = channel_input.trim().to_string();
        if crate::channel::validation::validate_channel_name(&channel).is_ok() {
            config.default_channel = channel;
        } else {
            println!("⚠️  Invalid channel name, using default");
        }
    }
    Ok(())
}

fn collect_log_level_value(config: &mut crate::models::Configuration) -> Result<()> {
    use crate::models::LogLevel;

    print!(
        "Log level (error/warn/info/debug/trace) [{}]: ",
        match config.log_level {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    );
    io::stdout().flush()?;
    let log_level_input = read_user_input_sync()?;
    if !log_level_input.trim().is_empty() {
        config.log_level = match log_level_input.trim().to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => {
                println!("⚠️  Invalid log level, using default");
                config.log_level.clone()
            }
        };
    }
    Ok(())
}

fn collect_host_value(config: &mut crate::models::Configuration) -> Result<()> {
    print!("Server bind address [{}]: ", config.server_host);
    io::stdout().flush()?;
    let host_input = read_user_input_sync()?;
    if !host_input.trim().is_empty() {
        config.server_host = host_input.trim().to_string();
    }
    Ok(())
}

fn collect_port_value(config: &mut crate::models::Configuration) -> Result<()> {
    print!("Server port [{}]: ", config.server_port);
    io::stdout().flush()?;
    let port_input = read_user_input_sync()?;
    if !port_input.trim().is_empty() {
        if let Ok(port) = port_input.trim().parse::<u16>() {
            config.server_port = port;
        } else {
            println!("⚠️  Invalid port number, using default");
        }
    }
    Ok(())
}

fn validate_and_save_config(
    config: &crate::models::Configuration,
    config_path: &std::path::Path,
) -> Result<()> {
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

    Ok(())
}

fn display_config_summary(config: &crate::models::Configuration) {
    println!("✅ Configuration saved successfully!");
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

fn display_url_image(image_path: &str, channel: &str) {
    println!("🖼️  [{}] Image URL: {}", channel, image_path);
    println!("💡 Please open this URL in your browser to view the image:");
    println!("   {}", image_path);
}

fn display_file_image(image_path: &str, channel: &str) -> Result<()> {
    let path = std::path::Path::new(image_path);
    if path.exists() {
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
    } else {
        return Err(anyhow::anyhow!("Image file not found: {}", image_path));
    }

    Ok(())
}

fn validate_url(url: &str) -> Result<()> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow::anyhow!(
            "Invalid URL format. Must start with http:// or https://"
        ));
    }
    Ok(())
}

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

/// Authorization decision types
enum AuthorizationDecision {
    Approved,
    Denied,
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
