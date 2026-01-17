//! HTTP API server for web clients

use crate::models::Message;
use crate::server::broadcast::BroadcastManager;
use crate::server::history::MessageHistory;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use warp::Filter;

/// API error types
#[derive(Debug)]
pub enum ApiError {
    ValidationError(String),
}

impl warp::reject::Reject for ApiError {}

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

/// Health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub active_connections: usize,
    pub queue_size: usize,
    pub active_channels: usize,
}

/// Response request for POST /api/v1/messages/:id/response
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseRequest {
    pub answer: Option<String>,
    pub response_type: crate::models::ResponseType,
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

    // GET /api/v1/health - Health check endpoint
    let get_health = warp::path!("api" / "v1" / "health")
        .and(warp::get())
        .and(message_history_filter.clone())
        .and(broadcast_manager_filter.clone())
        .and_then(handle_get_health);

    // POST /api/test - Test endpoint
    let post_test = warp::path!("api" / "test")
        .and(warp::post())
        .map(|| warp::reply::json(&serde_json::json!({"test": "ok"})));

    // POST /api/v1/messages - Send a message
    let post_messages = warp::path!("api" / "v1" / "messages")
        .and(warp::post())
        .and(warp::body::json())
        .and(message_history_filter.clone())
        .and(broadcast_manager_filter.clone())
        .and_then(handle_post_messages);

    // GET /api/v1/messages/:id - Get message by ID
    let get_message = warp::path!("api" / "v1" / "messages" / Uuid)
        .and(warp::get())
        .and(message_history_filter.clone())
        .and_then(handle_get_message);

    // POST /api/v1/messages/:id/response - Send response to message
    let post_response = warp::path!("api" / "v1" / "messages" / Uuid / "response")
        .and(warp::post())
        .and(warp::body::json())
        .and(message_history_filter.clone())
        .and(broadcast_manager_filter.clone())
        .and_then(handle_post_response);

    post_test
        .or(get_channels)
        .or(get_channel_messages)
        .or(get_channel_stats)
        .or(get_stats)
        .or(get_health)
        .or(post_messages)
        .or(get_message)
        .or(post_response)
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

/// Handle GET /api/v1/health
async fn handle_get_health(
    _message_history: Arc<MessageHistory>,
    broadcast_manager: Arc<BroadcastManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Get health metrics from broadcast manager
    let broadcast_stats = broadcast_manager.get_stats().await;

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        active_connections: broadcast_stats.total_viewers,
        queue_size: 0, // TODO: Get from ChannelManager when available
        active_channels: broadcast_stats.active_channels,
    };

    Ok(warp::reply::json(&response))
}

/// Handle POST /api/messages (with bytes)
async fn handle_post_messages_bytes(
    body: bytes::Bytes,
    message_history: Arc<MessageHistory>,
    broadcast_manager: Arc<BroadcastManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Parse JSON manually
    let message: Message = serde_json::from_slice(&body).map_err(|e| {
        warp::reject::custom(ApiError::ValidationError(format!("Invalid JSON: {}", e)))
    })?;

    // Validate channel name
    crate::channel::validation::validate_channel_name(&message.channel)
        .map_err(|e| warp::reject::custom(ApiError::ValidationError(e.to_string())))?;

    // Add message to history
    message_history
        .add_message(&message.channel, message.clone())
        .await;

    // Broadcast the message
    broadcast_manager.broadcast_message(&message).await;

    // Return 201 Created with the message
    Ok(warp::reply::with_status(
        warp::reply::json(&message),
        warp::http::StatusCode::CREATED,
    ))
}

/// Handle POST /api/v1/messages
async fn handle_post_messages(
    message: Message,
    message_history: Arc<MessageHistory>,
    broadcast_manager: Arc<BroadcastManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Validate channel name
    crate::channel::validation::validate_channel_name(&message.channel)
        .map_err(|e| warp::reject::custom(ApiError::ValidationError(e.to_string())))?;

    // Add message to history
    message_history
        .add_message(&message.channel, message.clone())
        .await;

    // Broadcast the message
    broadcast_manager.broadcast_message(&message).await;

    // Return 201 Created with the message
    Ok(warp::reply::with_status(
        warp::reply::json(&message),
        warp::http::StatusCode::CREATED,
    ))
}

/// Handle GET /api/v1/messages/:id
async fn handle_get_message(
    message_id: Uuid,
    message_history: Arc<MessageHistory>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Look up message by ID
    if let Some(message) = message_history.get_message_by_id(&message_id).await {
        Ok(warp::reply::with_status(
            warp::reply::json(&message),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Message not found",
                "message_id": message_id.to_string()
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

/// Handle POST /api/v1/messages/:id/response
async fn handle_post_response(
    message_id: Uuid,
    response_request: ResponseRequest,
    message_history: Arc<MessageHistory>,
    broadcast_manager: Arc<BroadcastManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Look up the original message
    let original_message = match message_history.get_message_by_id(&message_id).await {
        Some(msg) => msg,
        None => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Original message not found",
                    "message_id": message_id.to_string()
                })),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    };

    // Create response message
    let response_content = crate::models::MessageContent::Response {
        answer: response_request.answer,
        response_type: response_request.response_type,
    };

    let response_message = crate::models::Message::response(
        original_message.channel.clone(),
        response_content,
        message_id, // correlation_id points to original message
    );

    // Add response to history
    message_history
        .add_message(&response_message.channel, response_message.clone())
        .await;

    // Broadcast the response
    broadcast_manager.broadcast_message(&response_message).await;

    // Return the response message
    Ok(warp::reply::with_status(
        warp::reply::json(&response_message),
        warp::http::StatusCode::OK,
    ))
}
