//! Handler for the `ailoop queue` subcommand.

use super::queue::QueueArgs;
use super::task_handlers::resolve_server_url;
use ailoop_core::PendingClient;
use anyhow::Result;

pub async fn handle_queue_commands(args: QueueArgs) -> Result<()> {
    let server_url = resolve_server_url(args.server)?;
    let client = PendingClient::new(&server_url);
    let response = client.list_pending(args.channel.as_deref()).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    let filter_label = match &args.channel {
        Some(ch) => format!("channel={}", ch),
        None => "all channels".to_string(),
    };

    println!(
        "Human queue: {} pending ({})",
        response.total_count, filter_label
    );
    println!();

    if response.items.is_empty() {
        println!("(no pending items)");
        return Ok(());
    }

    let terminal_width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80);

    let fixed_cols_width = 2 + 2 + 4 + 2 + 10 + 2 + 10 + 2;
    let title_width = terminal_width.saturating_sub(fixed_cols_width).max(10);

    println!(
        "{:<2}  {:<2}  {:<10}  {:<10}  Title",
        "#", "Ty", "Channel", "Msg"
    );

    for item in &response.items {
        let ty_char = match item.kind.as_str() {
            "decision" => "D",
            "authorize" => "A",
            "navigate" => "N",
            _ => "?",
        };

        let channel_display: String = item.channel.chars().take(10).collect();

        let msg_id_str = item.message_id.to_string().replace('-', "");
        let msg_display = format!("{}...", &msg_id_str[..8]);

        let title_display: String = item.label.chars().take(title_width).collect();

        println!(
            "{:<2}  {:<2}  {:<10}  {:<10}  {}",
            item.position, ty_char, channel_display, msg_display, title_display
        );
    }

    Ok(())
}
