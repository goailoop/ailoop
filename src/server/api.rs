//! HTTP API server for web clients

use crate::server::history::MessageHistory;
use crate::server::broadcast::BroadcastManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::Filter;

/// API response for channel information
#[derive(Debug, Clone, Serialize)]
pub struct ChannelInfo {
    pub name: String,
    pub message_count: usize,
    pub oldest_message: Option<String>,
    pub newest_message: Option<String>,
}

/// API response for channel list
#[derive(Debug, Clone, Serialize)]
pub struct ChannelsResponse {
    pub channels: Vec<ChannelInfo>,
}

/// API response for message history
#[derive(Debug, Clone, Serialize)]
pub struct MessagesResponse {
    pub channel: String,
    pub messages: Vec<serde_json::Value>,
    pub total_count: usize,
}

/// API response for channel statistics
#[derive(Debug, Clone, Serialize)]
pub struct StatsResponse {
    pub channel: String,
    pub message_count: usize,
    pub oldest_message: Option<String>,
    pub newest_message: Option<String>,
}

/// Create HTTP API routes
pub fn create_api_routes(
    message_history: Arc<MessageHistory>,
    broadcast_manager: Arc<BroadcastManager>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let message_history_filter = warp::any().map(move || Arc::clone(&message_history));
    let broadcast_manager_filter = warp::any().map(move || Arc::clone(&broadcast_manager));

    // GET /api/channels - List all channels
    let get_channels = warp::path!("api" / "channels")
        .and(warp::get())
        .and(message_history_filter.clone())
        .and_then(handle_get_channels);

    // GET /api/channels/:channel/messages - Get message history for a channel
    let get_channel_messages = warp::path!("api" / "channels" / String / "messages")
        .and(warp::get())
        .and(warp::query::<MessagesQuery>())
        .and(message_history_filter.clone())
        .and_then(handle_get_channel_messages);

    // GET /api/channels/:channel/stats - Get statistics for a channel
    let get_channel_stats = warp::path!("api" / "channels" / String / "stats")
        .and(warp::get())
        .and(message_history_filter.clone())
        .and_then(handle_get_channel_stats);

    // GET /api/stats - Get overall broadcast statistics
    let get_stats = warp::path!("api" / "stats")
        .and(warp::get())
        .and(broadcast_manager_filter.clone())
        .and_then(handle_get_stats);

    get_channels
        .or(get_channel_messages)
        .or(get_channel_stats)
        .or(get_stats)
}

/// Query parameters for message history
#[derive(Debug, Deserialize)]
struct MessagesQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

/// Handle GET /api/channels
async fn handle_get_channels(
    message_history: Arc<MessageHistory>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let channels = message_history.get_channels().await;

    let mut channel_infos = Vec::new();
    for channel_name in channels {
        let stats = message_history.get_channel_stats(&channel_name).await;
        let info = ChannelInfo {
            name: channel_name,
            message_count: stats.message_count,
            oldest_message: stats.oldest_message.map(|dt| dt.to_rfc3339()),
            newest_message: stats.newest_message.map(|dt| dt.to_rfc3339()),
        };
        channel_infos.push(info);
    }

    let response = ChannelsResponse {
        channels: channel_infos,
    };

    Ok(warp::reply::json(&response))
}

/// Handle GET /api/channels/:channel/messages
async fn handle_get_channel_messages(
    channel: String,
    query: MessagesQuery,
    message_history: Arc<MessageHistory>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let limit = query.limit.unwrap_or(100);
    let messages = message_history.get_messages(&channel, Some(limit)).await;

    // Convert messages to JSON values for API response
    let message_values: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|msg| serde_json::to_value(msg).unwrap_or_else(|_| serde_json::json!({})))
        .collect();

    let response = MessagesResponse {
        channel,
        messages: message_values.clone(),
        total_count: message_values.len(),
    };

    Ok(warp::reply::json(&response))
}

/// Handle GET /api/channels/:channel/stats
async fn handle_get_channel_stats(
    channel: String,
    message_history: Arc<MessageHistory>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let stats = message_history.get_channel_stats(&channel).await;

    let response = StatsResponse {
        channel,
        message_count: stats.message_count,
        oldest_message: stats.oldest_message.map(|dt| dt.to_rfc3339()),
        newest_message: stats.newest_message.map(|dt| dt.to_rfc3339()),
    };

    Ok(warp::reply::json(&response))
}

/// Handle GET /api/stats
async fn handle_get_stats(
    broadcast_manager: Arc<BroadcastManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let stats = broadcast_manager.get_stats().await;
    Ok(warp::reply::json(&stats))
}