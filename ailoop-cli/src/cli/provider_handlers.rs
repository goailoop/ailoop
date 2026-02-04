//! Handler for provider list and provider telegram test

use anyhow::Result;
use std::path::PathBuf;

use super::provider::{ProviderCommands, TelegramCommands};
use ailoop_core::models::{Configuration, Message, MessageContent, SenderType};
use ailoop_core::server::providers::NotificationSink;

fn resolve_config_path(config_arg: &str) -> Result<PathBuf> {
    if config_arg.starts_with("~/") {
        let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME not set"))?;
        Ok(PathBuf::from(config_arg.replacen(
            "~/",
            &format!("{}/", home),
            1,
        )))
    } else if config_arg == "~/.config/ailoop/config.toml" {
        Configuration::default_config_path().map_err(|e| anyhow::anyhow!("Config path: {}", e))
    } else {
        Ok(PathBuf::from(config_arg))
    }
}

fn load_config(config_arg: &str) -> Result<Configuration> {
    let path = resolve_config_path(config_arg)?;
    Ok(Configuration::load_from_file(&path).unwrap_or_default())
}

/// Status for provider list (no secrets)
fn telegram_status(config: &Configuration) -> &'static str {
    if !config.providers.telegram.enabled {
        return "disabled";
    }
    let chat_ok = config
        .providers
        .telegram
        .chat_id
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let token_set = std::env::var("AILOOP_TELEGRAM_BOT_TOKEN").is_ok();
    match (token_set, chat_ok) {
        (true, true) => "configured",
        (false, _) => "missing_token",
        (_, false) => "missing_chat_id",
    }
}

pub async fn handle_provider_commands(command: ProviderCommands) -> Result<()> {
    match command {
        ProviderCommands::List { config } => handle_provider_list(&config).await,
        ProviderCommands::Telegram { command: cmd } => match cmd {
            TelegramCommands::Test { config } => handle_provider_telegram_test(&config).await,
        },
    }
}

async fn handle_provider_list(config_arg: &str) -> Result<()> {
    let config = load_config(config_arg)?;
    let status = telegram_status(&config);
    println!("provider\tenabled\tstatus");
    println!(
        "telegram\t{}\t{}",
        config.providers.telegram.enabled, status
    );
    Ok(())
}

async fn handle_provider_telegram_test(config_arg: &str) -> Result<()> {
    let config = load_config(config_arg)?;
    if !config.providers.telegram.enabled {
        eprintln!("Telegram not enabled in config");
        std::process::exit(1);
    }
    let chat_id = config
        .providers
        .telegram
        .chat_id
        .as_ref()
        .filter(|s| !s.is_empty())
        .cloned();
    let token = std::env::var("AILOOP_TELEGRAM_BOT_TOKEN").ok();
    let (token, chat_id) = match (token, chat_id) {
        (Some(t), Some(c)) => (t, c),
        (None, _) => {
            eprintln!("AILOOP_TELEGRAM_BOT_TOKEN not set");
            std::process::exit(1);
        }
        (_, None) => {
            eprintln!("Chat ID not configured for Telegram");
            std::process::exit(1);
        }
    };
    let sink = ailoop_core::server::providers::TelegramSink::new(token, chat_id);
    let msg = Message::new(
        "public".to_string(),
        SenderType::Agent,
        MessageContent::Notification {
            text: "ailoop Telegram test".to_string(),
            priority: ailoop_core::models::NotificationPriority::Normal,
        },
    );
    sink.send(&msg)
        .await
        .map_err(|e| anyhow::anyhow!("Send failed: {}", e))?;
    println!("Test message sent to Telegram");
    Ok(())
}
