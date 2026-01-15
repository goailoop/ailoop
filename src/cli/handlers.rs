//! CLI command handlers

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
    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Determine operation mode
    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    // If server mode, send message via WebSocket and wait for response
    if operation_mode.is_server() {
        let server_url = operation_mode
            .server_url
            .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

        // Parse question for multiple choice (pipe-separated: "question|choice1|choice2|choice3")
        let (question_text, choices) = if question.contains('|') {
            let parts: Vec<&str> = question.split('|').collect();
            if parts.len() < 2 {
                return Err(anyhow::anyhow!(
                    "Invalid multiple choice format. Expected: 'question|choice1|choice2|...'"
                ));
            }
            let q_text = parts[0].trim().to_string();
            let choices_vec: Vec<String> =
                parts[1..].iter().map(|s| s.trim().to_string()).collect();
            (q_text, Some(choices_vec))
        } else {
            (question.clone(), None)
        };

        let choices_clone = choices.clone();

        // Create question message
        let content = crate::models::MessageContent::Question {
            text: question_text.clone(),
            timeout_seconds: timeout_secs,
            choices,
        };

        let message =
            crate::models::Message::new(channel.clone(), crate::models::SenderType::Agent, content);

        if !json {
            if choices_clone.is_some() {
                println!(
                    "📤 Sending multiple choice question to server: {}",
                    question_text
                );
            } else {
                println!("📤 Sending question to server: {}", question_text);
            }
            println!("⏳ Waiting for response...");
        }

        // Send message and wait for response
        let response = crate::transport::websocket::send_message_and_wait_response(
            server_url.clone(),
            channel.clone(),
            message,
            timeout_secs,
        )
        .await
        .context("Failed to communicate with server")?;

        match response {
            Some(response_msg) => {
                // Extract answer from response
                if let crate::models::MessageContent::Response {
                    answer,
                    response_type,
                } = &response_msg.content
                {
                    match response_type {
                        crate::models::ResponseType::Text => {
                            let answer_text = answer.as_deref().unwrap_or("(no answer provided)");
                            if json {
                                // Build JSON response with metadata if available
                                let mut json_response = serde_json::json!({
                                    "response": answer_text,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });

                                // Add metadata (index and value) if present (for multiple choice)
                                if let Some(metadata) = &response_msg.metadata {
                                    json_response["metadata"] = metadata.clone();
                                }

                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                // Display response with index if multiple choice
                                if let Some(metadata) = &response_msg.metadata {
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
                            return Ok(());
                        }
                        crate::models::ResponseType::Timeout => {
                            if json {
                                let json_response = serde_json::json!({
                                    "error": "timeout",
                                    "message": "Question timed out",
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!(
                                    "⏱️  Timeout: No response received within {} seconds",
                                    timeout_secs
                                );
                            }
                            std::process::exit(1);
                        }
                        crate::models::ResponseType::Cancelled => {
                            if json {
                                let json_response = serde_json::json!({
                                    "error": "cancelled",
                                    "message": "Question was cancelled",
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("⚠️  Question was cancelled");
                            }
                            std::process::exit(130);
                        }
                        _ => {
                            if json {
                                let json_response = serde_json::json!({
                                    "error": "unknown",
                                    "message": format!("Unexpected response type: {:?}", response_type),
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("⚠️  Unexpected response type: {:?}", response_type);
                            }
                            std::process::exit(1);
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Server sent unexpected message type"));
                }
            }
            None => {
                if json {
                    let json_response = serde_json::json!({
                        "error": "timeout",
                        "message": "No response received from server",
                        "channel": channel,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("⏱️  Timeout: No response received from server");
                }
                std::process::exit(1);
            }
        }
    }

    // Direct mode: display the question locally
    print!("❓ {}: ", question);
    io::stdout().flush().context("Failed to flush stdout")?;

    // Collect response with optional timeout and Ctrl+C handling
    let response = if timeout_secs > 0 {
        let timeout_duration = Duration::from_secs(timeout_secs as u64);
        tokio::select! {
            result = timeout(timeout_duration, read_user_input()) => {
                match result {
                    Ok(Ok(answer)) => answer,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        // Timeout occurred
                        if json {
                            let error_response = serde_json::json!({
                                "error": "timeout",
                                "message": format!("Question timed out after {} seconds", timeout_secs),
                                "channel": channel,
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            });
                            println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                        } else {
                            println!("\n⏱️  Timeout: No response received within {} seconds", timeout_secs);
                        }
                        std::process::exit(1);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                if json {
                    let error_response = serde_json::json!({
                        "error": "cancelled",
                        "message": "Question cancelled by user (Ctrl+C)",
                        "channel": channel,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                } else {
                    println!("\n⚠️  Cancelled by user (Ctrl+C)");
                }
                std::process::exit(130); // Standard exit code for SIGINT
            }
        }
    } else {
        // No timeout - wait indefinitely, but still handle Ctrl+C
        tokio::select! {
            result = read_user_input() => {
                result.context("Failed to read user input")?
            }
            _ = signal::ctrl_c() => {
                if json {
                    let error_response = serde_json::json!({
                        "error": "cancelled",
                        "message": "Question cancelled by user (Ctrl+C)",
                        "channel": channel,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                } else {
                    println!("\n⚠️  Cancelled by user (Ctrl+C)");
                }
                std::process::exit(130);
            }
        }
    };

    // Return response
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
    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Determine operation mode
    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    // If server mode, send message via WebSocket and wait for response
    if operation_mode.is_server() {
        let server_url = operation_mode
            .server_url
            .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

        // Create authorization message
        let content = crate::models::MessageContent::Authorization {
            action: action.clone(),
            context: None,
            timeout_seconds: timeout_secs,
        };

        let message =
            crate::models::Message::new(channel.clone(), crate::models::SenderType::Agent, content);

        if !json {
            println!("📤 Sending authorization request to server: {}", action);
            println!("⏳ Waiting for response...");
        }

        // Send message and wait for response
        let response = crate::transport::websocket::send_message_and_wait_response(
            server_url.clone(),
            channel.clone(),
            message,
            timeout_secs,
        )
        .await
        .context("Failed to communicate with server")?;

        match response {
            Some(response_msg) => {
                // Extract authorization decision from response
                if let crate::models::MessageContent::Response {
                    answer: _,
                    response_type,
                } = &response_msg.content
                {
                    match response_type {
                        crate::models::ResponseType::AuthorizationApproved => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": true,
                                    "action": action,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("✅ Authorization GRANTED");
                            }
                            return Ok(());
                        }
                        crate::models::ResponseType::AuthorizationDenied => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": false,
                                    "action": action,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("❌ Authorization DENIED");
                            }
                            std::process::exit(1);
                        }
                        crate::models::ResponseType::Timeout => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": false,
                                    "action": action,
                                    "channel": channel,
                                    "reason": "timeout",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("⏱️  Timeout: No response received. Defaulting to DENIED for security.");
                            }
                            std::process::exit(1);
                        }
                        crate::models::ResponseType::Cancelled => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": false,
                                    "action": action,
                                    "channel": channel,
                                    "reason": "cancelled",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("⚠️  Authorization was cancelled (skipped on server)");
                            }
                            std::process::exit(130); // Standard exit code for SIGINT/cancellation
                        }
                        _ => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": false,
                                    "action": action,
                                    "channel": channel,
                                    "reason": "unknown",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("⚠️  Unexpected response type: {:?}", response_type);
                            }
                            std::process::exit(1);
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Server sent unexpected message type"));
                }
            }
            None => {
                if json {
                    let json_response = serde_json::json!({
                        "authorized": false,
                        "action": action,
                        "channel": channel,
                        "reason": "timeout",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("⏱️  Timeout: No response received from server. Defaulting to DENIED for security.");
                }
                std::process::exit(1);
            }
        }
    }

    // Direct mode: display the authorization request locally
    println!("🔐 Authorization Request");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Action: {}", action);
    println!("Channel: {}", channel);
    if timeout_secs > 0 {
        println!("Timeout: {} seconds", timeout_secs);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print!("Authorize this action? (authorized/denied): ");
    io::stdout().flush().context("Failed to flush stdout")?;

    // Collect response with timeout (defaults to denial) and Ctrl+C handling
    let decision = if timeout_secs > 0 {
        let timeout_duration = Duration::from_secs(timeout_secs as u64);
        tokio::select! {
            result = timeout(timeout_duration, read_user_input()) => {
                match result {
                    Ok(Ok(answer)) => parse_authorization_response(&answer)?,
                    Ok(Err(_)) => {
                        // Read error - default to denial
                        AuthorizationDecision::Denied
                    }
                    Err(_) => {
                        // Timeout - default to denial for security
                        if json {
                            let error_response = serde_json::json!({
                                "authorized": false,
                                "action": action,
                                "channel": channel,
                                "reason": "timeout",
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            });
                            println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                        } else {
                            println!("\n⏱️  Timeout: No response received. Defaulting to DENIED for security.");
                        }
                        std::process::exit(1);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                // Ctrl+C - default to denial for security
                if json {
                    let error_response = serde_json::json!({
                        "authorized": false,
                        "action": action,
                        "channel": channel,
                        "reason": "cancelled",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                } else {
                    println!("\n⚠️  Cancelled by user (Ctrl+C). Defaulting to DENIED for security.");
                }
                std::process::exit(1);
            }
        }
    } else {
        // No timeout - wait for response, but handle Ctrl+C
        tokio::select! {
            result = read_user_input() => {
                let answer = result.context("Failed to read user input")?;
                parse_authorization_response(&answer)?
            }
            _ = signal::ctrl_c() => {
                // Ctrl+C - default to denial for security
                if json {
                    let error_response = serde_json::json!({
                        "authorized": false,
                        "action": action,
                        "channel": channel,
                        "reason": "cancelled",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                } else {
                    println!("\n⚠️  Cancelled by user (Ctrl+C). Defaulting to DENIED for security.");
                }
                std::process::exit(1);
            }
        }
    };

    // Return decision
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

    // Exit with appropriate code
    if authorized {
        Ok(())
    } else {
        std::process::exit(1);
    }
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
            // Invalid response - prompt again
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
    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Validate priority
    let priority_level = match priority.to_lowercase().as_str() {
        "low" => "low",
        "normal" => "normal",
        "high" => "high",
        "urgent" => "urgent",
        _ => {
            eprintln!("Warning: Invalid priority '{}', using 'normal'", priority);
            "normal"
        }
    };

    // Display notification
    let priority_icon = match priority_level {
        "urgent" => "🚨",
        "high" => "⚠️ ",
        "low" => "ℹ️ ",
        _ => "💬",
    };

    println!(
        "{} [{}] {}",
        priority_icon,
        priority_level.to_uppercase(),
        message
    );
    println!("📺 Channel: {}", channel);

    // In direct mode, notification is just displayed
    // In server mode, this would be sent to connected humans

    Ok(())
}

/// Handle the 'serve' command
pub async fn handle_serve(host: String, port: u16, channel: String) -> Result<()> {
    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Create and start server
    let server = crate::server::AiloopServer::new(host, port, channel);
    server.start().await
}

/// Handle the 'config' command
pub async fn handle_config_init(config_file: String) -> Result<()> {
    use crate::models::{Configuration, LogLevel};
    use std::path::PathBuf;

    println!("⚙️  Initializing ailoop configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Resolve config file path
    let config_path = if config_file.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        PathBuf::from(config_file.replacen("~/", &format!("{}/", home), 1))
    } else if config_file == "~/.config/ailoop/config.toml" {
        // Use XDG default
        Configuration::default_config_path()
            .map_err(|e| anyhow::anyhow!("Failed to get default config path: {}", e))?
    } else {
        PathBuf::from(config_file)
    };

    println!("📄 Config file: {}", config_path.display());

    // Check if config already exists
    let mut config = if config_path.exists() {
        println!("⚠️  Configuration file already exists. Loading existing values...");
        Configuration::load_from_file(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load existing config: {}", e))?
    } else {
        println!("✨ Creating new configuration with defaults...");
        Configuration::default()
    };

    // Interactive prompts
    println!("\n📝 Please answer the following questions (press Enter to use default):\n");

    // Default timeout
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

    // Default channel
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

    // Log level
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

    // Server host
    print!("Server bind address [{}]: ", config.server_host);
    io::stdout().flush()?;
    let host_input = read_user_input_sync()?;
    if !host_input.trim().is_empty() {
        config.server_host = host_input.trim().to_string();
    }

    // Server port
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

    // Validate configuration
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

    // Save configuration
    println!("\n💾 Saving configuration to {}...", config_path.display());
    config
        .save_to_file(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to save configuration: {}", e))?;

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

    Ok(())
}

/// Handle the 'image' command
pub async fn handle_image(image_path: String, channel: String, _server: String) -> Result<()> {
    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Check if it's a URL or file path
    let is_url = image_path.starts_with("http://") || image_path.starts_with("https://");

    if is_url {
        println!("🖼️  [{}] Image URL: {}", channel, image_path);
        println!("💡 Please open this URL in your browser to view the image:");
        println!("   {}", image_path);
    } else {
        // Check if file exists
        let path = std::path::Path::new(&image_path);
        if path.exists() {
            println!("🖼️  [{}] Image file: {}", channel, image_path);
            println!("💡 Image location: {}", path.canonicalize()?.display());

            // Try to determine image type
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
    }

    // In server mode, this would be sent to connected humans
    // In direct mode, we just display the information

    Ok(())
}

/// Handle the 'navigate' command
pub async fn handle_navigate(url: String, channel: String, server: String) -> Result<()> {
    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow::anyhow!(
            "Invalid URL format. Must start with http:// or https://"
        ));
    }

    // Determine operation mode
    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    // If server mode, send message via WebSocket
    if operation_mode.is_server() {
        let server_url = operation_mode
            .server_url
            .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

        // Create navigate message
        let content = crate::models::MessageContent::Navigate { url: url.clone() };

        let message =
            crate::models::Message::new(channel.clone(), crate::models::SenderType::Agent, content);

        // Send message to server (no response expected for navigate)
        crate::transport::websocket::send_message_no_response(
            server_url.clone(),
            channel.clone(),
            message,
        )
        .await
        .context("Failed to send navigate message to server")?;

        println!("📤 Navigation request sent to server: {}", url);
        return Ok(());
    }

    // Direct mode: display the navigation suggestion
    println!("🧭 [{}] Navigation suggestion", channel);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("URL: {}", url);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 Please navigate to this URL in your browser:");
    println!("   {}", url);

    // Try to open URL if possible (platform-specific)
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(&["/C", "start", "", &url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_ask_placeholder() {
        // This is just a placeholder test
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
    use crate::cli::forward::{execute_forward, ForwardConfig};
    use crate::parser::InputFormat;
    use crate::transport::factory::TransportType;
    use std::path::PathBuf;

    // Validate channel name
    crate::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Parse input format
    let input_format = match format.as_str() {
        "json" => InputFormat::Json,
        "stream-json" => InputFormat::StreamJson,
        "text" => InputFormat::Text,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid format: {}. Must be one of: json, stream-json, text",
                format
            ));
        }
    };

    // Parse transport type
    let transport_type = match transport.as_str() {
        "websocket" => {
            if url.is_none() {
                return Err(anyhow::anyhow!("WebSocket transport requires --url option"));
            }
            TransportType::WebSocket
        }
        "file" => {
            if output.is_none() {
                return Err(anyhow::anyhow!("File transport requires --output option"));
            }
            TransportType::File
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid transport: {}. Must be one of: websocket, file",
                transport
            ));
        }
    };

    // Build forward config
    let config = ForwardConfig {
        channel,
        agent_type,
        format: input_format,
        transport_type,
        url,
        file_path: output.map(PathBuf::from),
        client_id,
        input_file: input.map(PathBuf::from),
    };

    // Execute forward command
    execute_forward(config).await
}
