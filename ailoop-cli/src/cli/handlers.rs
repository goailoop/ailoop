//! CLI command handlers

use anyhow::{Context, Result};
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::signal;

/// Minimal serde-compatible struct for parsing --decision-json input.
#[derive(serde::Deserialize)]
struct DecisionInput {
    decision_id: String,
    summary: String,
    #[serde(default)]
    context_markdown: Option<String>,
    options: Vec<ailoop_core::models::DecisionOption>,
    #[serde(default)]
    recommendation: Option<ailoop_core::models::DecisionRecommendation>,
    #[serde(default)]
    timeout_seconds: u32,
}

/// Handle the 'ask' command
pub async fn handle_ask(
    payload: String,
    channel: String,
    timeout_secs: u32,
    server: String,
    json: bool,
) -> Result<()> {
    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Parse the decision JSON
    let input: DecisionInput =
        serde_json::from_str(&payload).map_err(|e| anyhow::anyhow!("Invalid --payload: {}", e))?;

    // Use CLI --timeout if non-zero, otherwise use the value from the JSON
    let effective_timeout = if timeout_secs > 0 {
        timeout_secs
    } else {
        input.timeout_seconds
    };

    // Validate decision options before sending
    ailoop_core::models::validate_decision(&input.options, &input.recommendation)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Determine operation mode
    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    // If server mode, send message via WebSocket and wait for response
    if operation_mode.is_server() {
        let server_url = operation_mode
            .server_url
            .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

        if !json {
            println!("Sending decision to server: {}", input.summary);
            println!("Waiting for response...");
        }

        // Send decision and wait for response
        let response = ailoop_core::client::ask_decision(
            &server_url,
            &channel,
            input.decision_id,
            input.summary,
            input.context_markdown,
            input.options,
            input.recommendation,
            effective_timeout,
        )
        .await
        .context("Failed to communicate with server")?;

        match response {
            Some(response_msg) => {
                // Extract answer from response
                if let ailoop_core::models::MessageContent::Response {
                    answer,
                    response_type,
                } = &response_msg.content
                {
                    match response_type {
                        ailoop_core::models::ResponseType::Text => {
                            let answer_text = answer.as_deref().unwrap_or("(no answer provided)");
                            if json {
                                // Build JSON response with metadata if available
                                let mut json_response = serde_json::json!({
                                    "response": answer_text,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });

                                // Add metadata (option_id, label, index) if present
                                if let Some(metadata) = &response_msg.metadata {
                                    json_response["metadata"] = metadata.clone();
                                }

                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                // Display response with label and index if available
                                if let Some(metadata) = &response_msg.metadata {
                                    if let (Some(index), Some(label)) = (
                                        metadata.get("index").and_then(|v| v.as_u64()),
                                        metadata.get("label").and_then(|v| v.as_str()),
                                    ) {
                                        println!(
                                            "Decision resolved: {} (option #{}: {})",
                                            answer_text,
                                            index + 1,
                                            label
                                        );
                                    } else {
                                        println!("Decision resolved: {}", answer_text);
                                    }
                                } else {
                                    println!("Decision resolved: {}", answer_text);
                                }
                            }
                            return Ok(());
                        }
                        ailoop_core::models::ResponseType::Timeout => {
                            if json {
                                let json_response = serde_json::json!({
                                    "error": "timeout",
                                    "message": "Decision timed out",
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!(
                                    "Timeout: No response received within {} seconds",
                                    effective_timeout
                                );
                            }
                            return Err(anyhow::anyhow!("Decision timed out"));
                        }
                        ailoop_core::models::ResponseType::Cancelled => {
                            if json {
                                let json_response = serde_json::json!({
                                    "error": "cancelled",
                                    "message": "Decision was cancelled",
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("Decision was cancelled");
                            }
                            return Err(anyhow::anyhow!("Decision cancelled"));
                        }
                        ailoop_core::models::ResponseType::AuthorizationApproved
                        | ailoop_core::models::ResponseType::AuthorizationDenied => {
                            let default_answer = if matches!(
                                response_type,
                                ailoop_core::models::ResponseType::AuthorizationApproved
                            ) {
                                "yes"
                            } else {
                                "no"
                            };
                            let answer_text = answer.as_deref().unwrap_or(default_answer);
                            if json {
                                let mut json_response = serde_json::json!({
                                    "response": answer_text,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                if let Some(metadata) = &response_msg.metadata {
                                    json_response["metadata"] = metadata.clone();
                                }
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("Response received: {}", answer_text);
                            }
                            return Ok(());
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
                    println!("Timeout: No response received from server");
                }
                return Err(anyhow::anyhow!("No response received from server"));
            }
        }
    } else {
        // Direct mode: display decision locally and read user selection
        println!("Decision: {}", input.summary);
        println!("Options:");
        let rec_id = input.recommendation.as_ref().map(|r| r.option_id.as_str());
        for (idx, opt) in input.options.iter().enumerate() {
            let is_rec = rec_id == Some(opt.id.as_str());
            let rec_mark = if is_rec { " [recommended]" } else { "" };
            if let Some(detail) = &opt.detail_markdown {
                let truncated: String = detail.chars().take(80).collect();
                println!("  {}. {}{} — {}", idx + 1, opt.label, rec_mark, truncated);
            } else {
                println!("  {}. {}{}", idx + 1, opt.label, rec_mark);
            }
        }
        print!("Enter option id, label, or number: ");
        io::stdout().flush().context("Failed to flush stdout")?;

        let response = tokio::select! {
            result = read_user_input() => {
                result.context("Failed to read user input")?
            }
            _ = signal::ctrl_c() => {
                if json {
                    let error_response = serde_json::json!({
                        "error": "cancelled",
                        "message": "Decision cancelled by user (Ctrl+C)",
                        "channel": channel,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                } else {
                    println!("\nCancelled by user (Ctrl+C)");
                }
                return Err(anyhow::anyhow!("Cancelled by user"));
            }
        };

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

async fn stop_terminal_input<T>(
    cancelled: &Arc<AtomicBool>,
    handle: &mut tokio::task::JoinHandle<Result<T>>,
) {
    cancelled.store(true, Ordering::Relaxed);
    let _ = handle.await;
}

/// Handle the 'authorize' command
pub async fn handle_authorize(
    action: String,
    channel: String,
    timeout_secs: u32,
    server: String,
    json: bool,
    default_yes: bool,
) -> Result<()> {
    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Determine operation mode
    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    // If server mode, send message via WebSocket and wait for response
    if operation_mode.is_server() {
        let server_url = operation_mode
            .server_url
            .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

        if !json {
            println!("Sending authorization request to server: {}", action);
            println!("Waiting for response...");
        }

        // Send message and wait for response
        let response = ailoop_core::client::authorize(&server_url, &channel, &action, timeout_secs)
            .await
            .context("Failed to communicate with server")?;

        match response {
            Some(response_msg) => {
                // Extract authorization decision from response
                if let ailoop_core::models::MessageContent::Response {
                    answer: _,
                    response_type,
                } = &response_msg.content
                {
                    match response_type {
                        ailoop_core::models::ResponseType::AuthorizationApproved => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": true,
                                    "action": action,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("Authorization GRANTED");
                            }
                            return Ok(());
                        }
                        ailoop_core::models::ResponseType::AuthorizationDenied => {
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": false,
                                    "action": action,
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else {
                                println!("Authorization DENIED");
                            }
                            return Err(anyhow::anyhow!("Authorization denied"));
                        }
                        ailoop_core::models::ResponseType::Timeout => {
                            let decision = timeout_decision(default_yes);
                            let authorized = matches!(decision, AuthorizationDecision::Approved);
                            if json {
                                let json_response = serde_json::json!({
                                    "authorized": authorized,
                                    "action": action,
                                    "channel": channel,
                                    "reason": "timeout",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                println!("{}", serde_json::to_string_pretty(&json_response)?);
                            } else if authorized {
                                println!("Timeout: No response received. Defaulting to GRANTED (--default yes).");
                            } else {
                                println!("Timeout: No response received. Defaulting to DENIED (--default no).");
                            }
                            if authorized {
                                return Ok(());
                            } else {
                                return Err(anyhow::anyhow!("Authorization timed out"));
                            }
                        }
                        ailoop_core::models::ResponseType::Cancelled => {
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
                                println!("Authorization was cancelled (skipped on server)");
                            }
                            return Err(anyhow::anyhow!("Authorization cancelled"));
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
                                println!("Unexpected response type: {:?}", response_type);
                            }
                            return Err(anyhow::anyhow!(
                                "Unexpected authorization response type: {:?}",
                                response_type
                            ));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Server sent unexpected message type"));
                }
            }
            None => {
                let decision = timeout_decision(default_yes);
                let authorized = matches!(decision, AuthorizationDecision::Approved);
                if json {
                    let json_response = serde_json::json!({
                        "authorized": authorized,
                        "action": action,
                        "channel": channel,
                        "reason": "timeout",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else if authorized {
                    println!("Timeout: No response received from server. Defaulting to GRANTED (--default yes).");
                } else {
                    println!("Timeout: No response received from server. Defaulting to DENIED (--default no).");
                }
                if authorized {
                    return Ok(());
                } else {
                    return Err(anyhow::anyhow!("Authorization timed out"));
                }
            }
        }
    }

    // Direct mode: display the authorization request locally
    let is_tty = io::stdin().is_terminal() && io::stdout().is_terminal();
    println!("Authorization Request");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Action: {}", action);
    println!("Channel: {}", channel);
    if timeout_secs > 0 && !is_tty {
        println!("Timeout: {} seconds", timeout_secs);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let prompt = if default_yes {
        "Authorize this action? YES (ENTER) | no: "
    } else {
        "Authorize this action? yes | NO (ENTER): "
    };
    print!("{}", prompt);
    io::stdout().flush().context("Failed to flush stdout")?;

    let decision = if timeout_secs > 0 {
        let timeout_duration = Duration::from_secs(timeout_secs as u64);
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut input_task = tokio::task::spawn_blocking({
            let cancelled = Arc::clone(&cancelled);
            move || {
                crate::cli::terminal_input::read_user_input_with_countdown(
                    timeout_duration,
                    cancelled,
                )
            }
        });
        tokio::select! {
            result = &mut input_task => {
                match result {
                    Ok(Ok(ailoop_core::terminal::countdown::InputResult::Submitted(answer))) => {
                        parse_authorization_response(&answer, default_yes)?
                    }
                    Ok(Ok(ailoop_core::terminal::countdown::InputResult::Timeout)) => {
                        let decision = timeout_decision(default_yes);
                        let authorized = matches!(decision, AuthorizationDecision::Approved);
                        if json {
                            let error_response = serde_json::json!({
                                "authorized": authorized,
                                "action": action,
                                "channel": channel,
                                "reason": "timeout",
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            });
                            println!("\n{}", serde_json::to_string_pretty(&error_response)?);
                        } else if authorized {
                            println!("\nTimeout: No response received. Defaulting to GRANTED (--default yes).");
                        } else {
                            println!("\nTimeout: No response received. Defaulting to DENIED (--default no).");
                        }
                        if authorized {
                            return Ok(());
                        } else {
                            return Err(anyhow::anyhow!("Authorization timed out"));
                        }
                    }
                    Ok(Ok(ailoop_core::terminal::countdown::InputResult::Cancelled)) => {
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
                            println!("\nCancelled by user. Defaulting to DENIED for security.");
                        }
                        return Err(anyhow::anyhow!("Authorization cancelled"));
                    }
                    Ok(Err(_)) => {
                        AuthorizationDecision::Denied
                    }
                    Err(_) => {
                        AuthorizationDecision::Denied
                    }
                }
            }
            _ = signal::ctrl_c() => {
                stop_terminal_input(&cancelled, &mut input_task).await;
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
                    println!("\nCancelled by user (Ctrl+C). Defaulting to DENIED for security.");
                }
                return Err(anyhow::anyhow!("Authorization cancelled"));
            }
        }
    } else {
        tokio::select! {
            result = read_user_input() => {
                let answer = result.context("Failed to read user input")?;
                parse_authorization_response(&answer, default_yes)?
            }
            _ = signal::ctrl_c() => {
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
                    println!("\nCancelled by user (Ctrl+C). Defaulting to DENIED for security.");
                }
                return Err(anyhow::anyhow!("Authorization cancelled"));
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
        println!("Authorization GRANTED");
    } else {
        println!("Authorization DENIED");
    }

    // Exit with appropriate code
    if authorized {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Authorization denied"))
    }
}

/// Authorization decision types
enum AuthorizationDecision {
    Approved,
    Denied,
}

/// Maps a timeout event to an authorization decision based on the configured default.
/// `--default yes` + timeout => Approved; `--default no` + timeout => Denied.
fn timeout_decision(default_yes: bool) -> AuthorizationDecision {
    if default_yes {
        AuthorizationDecision::Approved
    } else {
        AuthorizationDecision::Denied
    }
}

/// Parse user input for authorization response
fn parse_authorization_response(input: &str, default_yes: bool) -> Result<AuthorizationDecision> {
    let normalized = input.trim().to_lowercase();

    match normalized.as_str() {
        "" => Ok(if default_yes {
            AuthorizationDecision::Approved
        } else {
            AuthorizationDecision::Denied
        }),
        "authorized" | "yes" | "y" | "approve" | "ok" => Ok(AuthorizationDecision::Approved),
        "denied" | "no" | "n" | "deny" | "reject" => Ok(AuthorizationDecision::Denied),
        _ => {
            let retry_prompt = if default_yes {
                "Invalid response. Please enter YES (ENTER) | no: "
            } else {
                "Invalid response. Please enter yes | NO (ENTER): "
            };
            print!("{}", retry_prompt);
            io::stdout().flush().context("Failed to flush stdout")?;
            let retry = read_user_input_sync()?;
            parse_authorization_response(&retry, default_yes)
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
    server: String,
) -> Result<()> {
    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Normalize priority for display and client usage
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

    // Determine operation mode
    let operation_mode = crate::mode::determine_operation_mode(Some(server))
        .map_err(|e| anyhow::anyhow!("Failed to determine operation mode: {}", e))?;

    if operation_mode.is_server() {
        let server_url = operation_mode
            .server_url
            .ok_or_else(|| anyhow::anyhow!("Server URL is required in server mode"))?;

        ailoop_core::client::say(&server_url, &channel, &message, priority_level)
            .await
            .context("Failed to send notification to server")?;

        println!(
            "Notification sent to server [{}]: {}",
            priority_level.to_uppercase(),
            message
        );
        println!("Channel: {}", channel);
        return Ok(());
    }

    // Display notification locally in direct mode
    let priority_label = match priority_level {
        "urgent" => "[URGENT]",
        "high" => "[HIGH]",
        "low" => "[LOW]",
        _ => "[INFO]",
    };

    println!("{} {}", priority_label, message);
    println!("Channel: {}", channel);

    Ok(())
}

/// Handle the 'serve' command
pub async fn handle_serve(host: String, port: u16, channel: String, web: bool) -> Result<()> {
    use ailoop_core::models::Configuration;
    use ailoop_server::{AiloopAppState, ServeConfig};
    use std::{net::SocketAddr, path::PathBuf, sync::Arc};
    use tokio_util::sync::CancellationToken;

    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Load config from default path (for provider settings)
    let config_path =
        Configuration::default_config_path().unwrap_or_else(|_| PathBuf::from("config.toml"));
    let provider_config = Configuration::load_from_file(&config_path).unwrap_or_default();

    let state =
        Arc::new(AiloopAppState::new(channel.clone()).with_provider_config(provider_config));

    let serve_config = ServeConfig {
        host: host.clone(),
        port,
        default_channel: channel.clone(),
        base_path: None,
        web,
        auth: None,
        cors: None,
    };

    let built_router = ailoop_server::router(Arc::clone(&state), &serve_config)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let address: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid server address: {}", e))?;

    println!("ailoop server starting on {}", address);
    println!("Default channel: {}", channel);
    println!("Press Ctrl+C to stop the server");
    if web {
        println!("Web UI available at http://{}:{}/", host, port);
    }

    let token = CancellationToken::new();
    let token_for_shutdown = token.clone();

    let task_handle =
        ailoop_server::spawn_background_tasks(Arc::clone(&state), &serve_config, token.clone());

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", address, e))?;

    axum::serve(listener, built_router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            token_for_shutdown.cancel();
        })
        .await?;

    token.cancel();
    let _ = task_handle.await;

    Ok(())
}

/// Handle the 'config' command
pub async fn handle_config_init(config_file: String) -> Result<()> {
    use ailoop_core::models::{Configuration, LogLevel};
    use std::path::PathBuf;

    println!("Initializing ailoop configuration");
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

    println!("Config file: {}", config_path.display());

    // Check if config already exists
    let mut config = if config_path.exists() {
        println!("Configuration file already exists. Loading existing values...");
        Configuration::load_from_file(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load existing config: {}", e))?
    } else {
        println!("Creating new configuration with defaults...");
        Configuration::default()
    };

    // Interactive prompts
    println!("\nPlease answer the following questions (press Enter to use default):\n");

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
            println!("Invalid timeout value, using default");
        }
    }

    // Default channel
    print!("Default channel name [{}]: ", config.default_channel);
    io::stdout().flush()?;
    let channel_input = read_user_input_sync()?;
    if !channel_input.trim().is_empty() {
        let channel = channel_input.trim().to_string();
        if ailoop_core::channel::validation::validate_channel_name(&channel).is_ok() {
            config.default_channel = channel;
        } else {
            println!("Invalid channel name, using default");
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
                println!("Invalid log level, using default");
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
            println!("Invalid port number, using default");
        }
    }

    // Telegram provider (optional)
    let tg_default = if config.providers.telegram.enabled {
        "y"
    } else {
        "n"
    };
    print!("Enable Telegram provider? (y/n) [{}]: ", tg_default);
    io::stdout().flush()?;
    let tg_input = read_user_input_sync()?;
    let enable_tg = if tg_input.trim().is_empty() {
        config.providers.telegram.enabled
    } else {
        matches!(tg_input.trim().to_lowercase().as_str(), "y" | "yes")
    };
    config.providers.telegram.enabled = enable_tg;
    if enable_tg {
        let chat_default = config.providers.telegram.chat_id.as_deref().unwrap_or("");
        print!(
            "Telegram chat ID (from @userinfobot or group) [{}]: ",
            chat_default
        );
        io::stdout().flush()?;
        let chat_input = read_user_input_sync()?;
        if !chat_input.trim().is_empty() {
            config.providers.telegram.chat_id = Some(chat_input.trim().to_string());
        }
        println!(
            "   Token must be set via environment (e.g. AILOOP_TELEGRAM_BOT_TOKEN); \
             it is not stored in the config file."
        );
    }

    // Validate configuration
    println!("\nValidating configuration...");
    match config.validate() {
        Ok(()) => {
            println!("Configuration is valid");
        }
        Err(errors) => {
            println!("Configuration validation failed:");
            for error in &errors {
                println!("   - {}", error);
            }
            return Err(anyhow::anyhow!("Configuration validation failed"));
        }
    }

    // Save configuration
    println!("\nSaving configuration to {}...", config_path.display());
    config
        .save_to_file(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to save configuration: {}", e))?;

    println!("Configuration saved successfully!");
    println!("\nConfiguration summary:");
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
    println!(
        "   Telegram: {}",
        if config.providers.telegram.enabled {
            format!(
                "enabled (chat_id: {})",
                config
                    .providers
                    .telegram
                    .chat_id
                    .as_deref()
                    .unwrap_or("not set")
            )
        } else {
            "disabled".to_string()
        }
    );

    Ok(())
}

/// Handle the 'image' command
pub async fn handle_image(image_path: String, channel: String, _server: String) -> Result<()> {
    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Check if it's a URL or file path
    let is_url = image_path.starts_with("http://") || image_path.starts_with("https://");

    if is_url {
        println!("[{}] Image URL: {}", channel, image_path);
        println!("Please open this URL in your browser to view the image:");
        println!("   {}", image_path);
    } else {
        // Check if file exists
        let path = std::path::Path::new(&image_path);
        if path.exists() {
            println!("[{}] Image file: {}", channel, image_path);
            println!("Image location: {}", path.canonicalize()?.display());

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
                println!("Image type: {}", img_type);
            }

            println!("Please open this file in an image viewer to view it.");
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
    ailoop_core::channel::validation::validate_channel_name(&channel)
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

        // Send message to server (no response expected for navigate)
        ailoop_core::client::navigate(&server_url, &channel, &url)
            .await
            .context("Failed to send navigate message to server")?;

        println!("Navigation request sent to server: {}", url);
        return Ok(());
    }

    // Direct mode: display the navigation suggestion
    println!("[{}] Navigation suggestion", channel);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("URL: {}", url);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Please navigate to this URL in your browser:");
    println!("   {}", url);

    // Try to open URL if possible (platform-specific)
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

    Ok(())
}

/// Handle the 'forward' command
#[allow(clippy::too_many_arguments)]
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
    use ailoop_core::transport::factory::TransportType;
    use std::path::PathBuf;

    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&channel)
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

/// Handle `ailoop config` (no flags) — display current config values.
pub async fn handle_config_show(config_file: String) -> Result<()> {
    use ailoop_core::models::Configuration;
    use std::path::PathBuf;

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

    if !config_path.exists() {
        println!("No config file found at {}", config_path.display());
        println!("Run `ailoop config --init` to create one.");
        return Ok(());
    }

    let config = Configuration::load_from_file(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    println!("Config: {}", config_path.display());
    println!("  server_host:      {}", config.server_host);
    println!("  server_port:      {}", config.server_port);
    println!("  default_channel:  {}", config.default_channel);
    println!("  telegram.enabled: {}", config.providers.telegram.enabled);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_ask_with_empty_server() {
        // Test that handle_ask falls back to direct mode when server is not provided.
        // Ensure the AILOOP_SERVER env var is unset in the test environment to avoid
        // flakiness when CI sets it.
        std::env::remove_var("AILOOP_SERVER");
        let operation_mode = crate::mode::determine_operation_mode(Some("".to_string()))
            .expect("Mode detection should succeed");

        // We expect direct mode when server is empty
        assert!(operation_mode.is_direct());
        assert!(!operation_mode.is_server());
        assert_eq!(operation_mode.server_url, None);
    }

    #[tokio::test]
    async fn test_handle_ask_with_server_flag_but_no_server_running() {
        // Test that handle_ask properly handles the case when --server flag is provided
        // but no server is actually running. This is the scenario from the bug report.
        // Use a non-routable, guaranteed-nonexistent domain to avoid depending on local
        // services during tests (prevents flakiness if a server happens to be running).
        let result = handle_ask(
            r#"{"decision_id":"test","summary":"What is your name?","options":[{"id":"a","label":"A"},{"id":"b","label":"B"}]}"#.to_string(), // payload
            "test-channel".to_string(),
            10,
            "http://nonexistent.invalid:12345".to_string(),
            false,
        )
        .await;

        // We expect this to fail with connection error since no server is running
        // The bug was that it should have fallen back to direct mode when the server flag
        // was not provided, but when it was provided, it should attempt connection and fail.
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Failed to communicate with server")
                || error_msg.contains("Failed to connect to WebSocket server")
                || error_msg.contains("No response received from server")
                || error_msg.contains("timed out")
        );
    }

    #[test]
    fn test_parse_authorization_response_empty_default_yes() {
        let result = parse_authorization_response("", true).unwrap();
        assert!(
            matches!(result, AuthorizationDecision::Approved),
            "Empty input with default_yes=true should return Approved"
        );
    }

    #[test]
    fn test_parse_authorization_response_empty_default_no() {
        let result = parse_authorization_response("", false).unwrap();
        assert!(
            matches!(result, AuthorizationDecision::Denied),
            "Empty input with default_yes=false should return Denied"
        );
    }

    #[test]
    fn test_parse_authorization_response_explicit_yes_overrides_default_no() {
        let result = parse_authorization_response("yes", false).unwrap();
        assert!(
            matches!(result, AuthorizationDecision::Approved),
            "Explicit 'yes' should override default_no"
        );
    }

    #[test]
    fn test_parse_authorization_response_explicit_no_overrides_default_yes() {
        let result = parse_authorization_response("no", true).unwrap();
        assert!(
            matches!(result, AuthorizationDecision::Denied),
            "Explicit 'no' should override default_yes"
        );
    }

    #[test]
    fn test_parse_authorization_response_all_approve_keywords() {
        let approve_keywords = vec!["authorized", "yes", "y", "approve", "ok"];
        for keyword in approve_keywords {
            for default in [true, false] {
                let result = parse_authorization_response(keyword, default).unwrap();
                assert!(
                    matches!(result, AuthorizationDecision::Approved),
                    "'{}' with default_yes={} should return Approved",
                    keyword,
                    default
                );
            }
        }
    }

    #[test]
    fn test_parse_authorization_response_all_deny_keywords() {
        let deny_keywords = vec!["denied", "no", "n", "deny", "reject"];
        for keyword in deny_keywords {
            for default in [true, false] {
                let result = parse_authorization_response(keyword, default).unwrap();
                assert!(
                    matches!(result, AuthorizationDecision::Denied),
                    "'{}' with default_yes={} should return Denied",
                    keyword,
                    default
                );
            }
        }
    }

    #[test]
    fn test_parse_authorization_response_whitespace_empty_default_yes() {
        let result = parse_authorization_response("   ", true).unwrap();
        assert!(
            matches!(result, AuthorizationDecision::Approved),
            "Whitespace-only input with default_yes=true should return Approved"
        );
    }

    #[test]
    fn test_parse_authorization_response_whitespace_empty_default_no() {
        let result = parse_authorization_response("   ", false).unwrap();
        assert!(
            matches!(result, AuthorizationDecision::Denied),
            "Whitespace-only input with default_yes=false should return Denied"
        );
    }

    #[test]
    fn test_timeout_decision_default_yes_returns_approved() {
        let decision = timeout_decision(true);
        assert!(
            matches!(decision, AuthorizationDecision::Approved),
            "timeout_decision(true) should return Approved"
        );
    }

    #[test]
    fn test_timeout_decision_default_no_returns_denied() {
        let decision = timeout_decision(false);
        assert!(
            matches!(decision, AuthorizationDecision::Denied),
            "timeout_decision(false) should return Denied"
        );
    }
}
