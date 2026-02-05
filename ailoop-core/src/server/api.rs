//! HTTP API server for web clients

use crate::models::{DependencyType, Message, Task, TaskState};
use crate::server::broadcast::BroadcastManager;
use crate::server::history::MessageHistory;
use crate::server::providers::PendingPromptRegistry;
use crate::server::task_storage::TaskStorage;
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

/// Request body for creating a task
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub channel: String,
    pub assignee: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Request body for updating a task state
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTaskRequest {
    pub state: TaskState,
}

/// Request body for adding a dependency
#[derive(Debug, Clone, Deserialize)]
pub struct AddDependencyRequest {
    pub child_id: Uuid,
    pub parent_id: Uuid,
    pub dependency_type: DependencyType,
}

/// Response for listing tasks
#[derive(Debug, Clone, Serialize)]
pub struct TasksResponse {
    pub channel: String,
    pub tasks: Vec<Task>,
    pub total_count: usize,
}

/// Create HTTP API routes
pub fn create_api_routes(
    message_history: Arc<MessageHistory>,
    broadcast_manager: Arc<BroadcastManager>,
    task_storage: Arc<TaskStorage>,
    pending_prompt_registry: Arc<PendingPromptRegistry>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let message_history_filter = warp::any().map(move || Arc::clone(&message_history));
    let broadcast_manager_filter = warp::any().map(move || Arc::clone(&broadcast_manager));
    let task_storage_filter = warp::any().map(move || Arc::clone(&task_storage));
    let pending_prompt_registry_filter =
        warp::any().map(move || Arc::clone(&pending_prompt_registry));

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
        .and(pending_prompt_registry_filter.clone())
        .and_then(handle_post_response);

    // POST /api/v1/tasks - Create task
    let post_tasks = warp::path!("api" / "v1" / "tasks")
        .and(warp::post())
        .and(warp::body::json())
        .and(task_storage_filter.clone())
        .and_then(handle_post_tasks);

    // GET /api/v1/tasks - List tasks
    let get_tasks = warp::path!("api" / "v1" / "tasks")
        .and(warp::get())
        .and(warp::query::<TaskQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_get_tasks);

    // GET /api/v1/tasks/:id - Get task details
    let get_task = warp::path!("api" / "v1" / "tasks" / Uuid)
        .and(warp::get())
        .and(warp::query::<TaskChannelQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_get_task);

    // PUT /api/v1/tasks/:id - Update task
    let put_task = warp::path!("api" / "v1" / "tasks" / Uuid)
        .and(warp::put())
        .and(warp::body::json())
        .and(warp::query::<TaskChannelQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_put_task);

    // POST /api/v1/tasks/:id/dependencies - Add dependency
    let post_task_dependencies = warp::path!("api" / "v1" / "tasks" / String / "dependencies")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::query::<TaskChannelQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_post_task_dependencies);

    // DELETE /api/v1/tasks/:id/dependencies/:dep_id - Remove dependency
    let delete_task_dependency =
        warp::path!("api" / "v1" / "tasks" / String / "dependencies" / Uuid)
            .and(warp::delete())
            .and(warp::query::<TaskChannelQuery>())
            .and(task_storage_filter.clone())
            .and_then(handle_delete_task_dependency);

    // GET /api/v1/tasks/ready - Get ready tasks
    let get_ready_tasks = warp::path!("api" / "v1" / "tasks" / "ready")
        .and(warp::get())
        .and(warp::query::<TaskQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_get_ready_tasks);

    // GET /api/v1/tasks/blocked - Get blocked tasks
    let get_blocked_tasks = warp::path!("api" / "v1" / "tasks" / "blocked")
        .and(warp::get())
        .and(warp::query::<TaskQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_get_blocked_tasks);

    // GET /api/v1/tasks/:id/dependencies - Get task dependencies
    let get_task_dependencies = warp::path!("api" / "v1" / "tasks" / Uuid / "dependencies")
        .and(warp::get())
        .and(warp::query::<TaskChannelQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_get_task_dependencies);

    // GET /api/v1/tasks/:id/graph - Get dependency graph
    let get_task_graph = warp::path!("api" / "v1" / "tasks" / Uuid / "graph")
        .and(warp::get())
        .and(warp::query::<TaskChannelQuery>())
        .and(task_storage_filter.clone())
        .and_then(handle_get_task_graph);

    post_test
        .or(get_channels)
        .or(get_channel_messages)
        .or(get_channel_stats)
        .or(get_stats)
        .or(get_health)
        .or(post_messages)
        .or(get_message)
        .or(post_response)
        .or(post_tasks)
        .or(get_tasks)
        .or(get_task)
        .or(put_task)
        .or(post_task_dependencies)
        .or(delete_task_dependency)
        .or(get_ready_tasks)
        .or(get_blocked_tasks)
        .or(get_task_dependencies)
        .or(get_task_graph)
}

/// Query parameters for message history
#[derive(Debug, Deserialize)]
struct MessagesQuery {
    limit: Option<usize>,
    _offset: Option<usize>,
}

/// Query parameters for task requests
#[derive(Debug, Deserialize)]
struct TaskQuery {
    channel: String,
    #[serde(rename = "state")]
    _state: Option<String>,
}

fn default_public_channel() -> String {
    "public".to_string()
}

#[derive(Debug, Deserialize)]
struct TaskChannelQuery {
    #[serde(default = "default_public_channel")]
    channel: String,
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
    pending_prompt_registry: Arc<PendingPromptRegistry>,
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

    let answer = response_request.answer.clone();
    let response_type = response_request.response_type.clone();

    // Create response message
    let response_content = crate::models::MessageContent::Response {
        answer: answer.clone(),
        response_type: response_type.clone(),
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

    // If a server-side prompt is waiting for this message, complete it so it doesn't time out.
    pending_prompt_registry
        .submit_reply_for_message(message_id, answer, response_type)
        .await;

    // Return the response message
    Ok(warp::reply::with_status(
        warp::reply::json(&response_message),
        warp::http::StatusCode::OK,
    ))
}

/// Handle POST /api/v1/tasks
async fn handle_post_tasks(
    request: CreateTaskRequest,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut task = Task::new(request.title, request.description);
    if let Some(assignee) = request.assignee {
        task = task.with_assignee(assignee);
    }
    if let Some(metadata) = request.metadata {
        task = task.with_metadata(metadata);
    }

    let created = task_storage
        .create_task(request.channel.clone(), task)
        .await
        .map_err(|e| {
            warp::reject::custom(ApiError::ValidationError(format!(
                "Failed to create task: {}",
                e
            )))
        })?;

    Ok(warp::reply::with_status(
        warp::reply::json(&created),
        warp::http::StatusCode::CREATED,
    ))
}

/// Handle GET /api/v1/tasks
async fn handle_get_tasks(
    query: TaskQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let state = query._state.and_then(|s| match s.as_str() {
        "pending" => Some(TaskState::Pending),
        "done" => Some(TaskState::Done),
        "abandoned" => Some(TaskState::Abandoned),
        _ => None,
    });

    let tasks = task_storage.list_tasks(&query.channel, state).await;

    let response = TasksResponse {
        channel: query.channel,
        tasks: tasks.clone(),
        total_count: tasks.len(),
    };

    Ok(warp::reply::json(&response))
}

/// Handle GET /api/v1/tasks/:id
async fn handle_get_task(
    task_id: Uuid,
    query: TaskChannelQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let task = task_storage.get_task(&query.channel, task_id).await;

    if let Some(task) = task {
        Ok(warp::reply::with_status(
            warp::reply::json(&task),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Task not found",
                "task_id": task_id.to_string()
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

/// Handle PUT /api/v1/tasks/:id
async fn handle_put_task(
    task_id: Uuid,
    request: UpdateTaskRequest,
    query: TaskChannelQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let task = task_storage
        .update_task_state(&query.channel, task_id, request.state)
        .await;

    match task {
        Ok(task) => Ok(warp::reply::with_status(
            warp::reply::json(&task),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": e.to_string(),
                "task_id": task_id.to_string()
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
    }
}

/// Handle POST /api/v1/tasks/:id/dependencies
async fn handle_post_task_dependencies(
    task_id: String,
    request: AddDependencyRequest,
    query: TaskChannelQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let child_id = Uuid::parse_str(&task_id).map_err(|_| {
        warp::reject::custom(ApiError::ValidationError("Invalid task ID".to_string()))
    })?;

    task_storage
        .add_dependency(
            query.channel,
            child_id,
            request.parent_id,
            request.dependency_type,
        )
        .await
        .map_err(|e| {
            warp::reject::custom(ApiError::ValidationError(format!(
                "Failed to add dependency: {}",
                e
            )))
        })?;

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({"status": "ok"})),
        warp::http::StatusCode::OK,
    ))
}

/// Handle DELETE /api/v1/tasks/:id/dependencies/:dep_id
async fn handle_delete_task_dependency(
    task_id: String,
    dep_id: Uuid,
    query: TaskChannelQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let child_id = Uuid::parse_str(&task_id).map_err(|_| {
        warp::reject::custom(ApiError::ValidationError("Invalid task ID".to_string()))
    })?;

    task_storage
        .remove_dependency(query.channel, child_id, dep_id)
        .await
        .map_err(|e| {
            warp::reject::custom(ApiError::ValidationError(format!(
                "Failed to remove dependency: {}",
                e
            )))
        })?;

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({"status": "ok"})),
        warp::http::StatusCode::OK,
    ))
}

/// Handle GET /api/v1/tasks/ready
async fn handle_get_ready_tasks(
    query: TaskQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let tasks = task_storage.get_ready_tasks(&query.channel).await;

    let response = TasksResponse {
        channel: query.channel,
        tasks: tasks.clone(),
        total_count: tasks.len(),
    };

    Ok(warp::reply::json(&response))
}

/// Handle GET /api/v1/tasks/blocked
async fn handle_get_blocked_tasks(
    query: TaskQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let tasks = task_storage.get_blocked_tasks(&query.channel).await;

    let response = TasksResponse {
        channel: query.channel,
        tasks: tasks.clone(),
        total_count: tasks.len(),
    };

    Ok(warp::reply::json(&response))
}

/// Handle GET /api/v1/tasks/:id/dependencies
async fn handle_get_task_dependencies(
    task_id: Uuid,
    query: TaskChannelQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(task) = task_storage.get_task(&query.channel, task_id).await {
        let dependencies: Vec<Uuid> = task.depends_on;
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "task_id": task_id,
                "depends_on": dependencies,
                "blocking_for": task.blocking_for
            })),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Task not found",
                "task_id": task_id.to_string()
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

/// Handle GET /api/v1/tasks/:id/graph
async fn handle_get_task_graph(
    task_id: Uuid,
    query: TaskChannelQuery,
    task_storage: Arc<TaskStorage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    match task_storage
        .get_dependency_graph(&query.channel, task_id)
        .await
    {
        Ok(graph) => Ok(warp::reply::with_status(
            warp::reply::json(&graph),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": e.to_string(),
                "task_id": task_id.to_string()
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
    }
}
