//! HTTP API server for web clients

use crate::server::core::AppState;
use ailoop_core::models::{DependencyType, Message, Task, TaskState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API error types
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Not found")]
    NotFound,
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str()),
        };
        let body = Json(serde_json::json!({"error": message}));
        (status, body).into_response()
    }
}

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
    pub response_type: ailoop_core::models::ResponseType,
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

/// Query parameters for GET /api/v1/pending
#[derive(Debug, Deserialize)]
struct PendingQuery {
    channel: Option<String>,
}

/// Per-item shape in the pending list response
#[derive(Debug, Clone, Serialize)]
pub struct PendingItemResponse {
    pub message_id: Uuid,
    pub kind: String,
    pub channel: String,
    pub position: usize,
    pub label: String,
}

/// Top-level response for GET /api/v1/pending
#[derive(Debug, Clone, Serialize)]
pub struct PendingListResponse {
    pub items: Vec<PendingItemResponse>,
    pub total_count: usize,
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

/// Create HTTP API router — state not yet bound
pub(crate) fn create_api_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/test", axum::routing::post(handle_post_test))
        .route("/api/channels", axum::routing::get(handle_get_channels))
        .route(
            "/api/channels/{channel}/messages",
            axum::routing::get(handle_get_channel_messages),
        )
        .route(
            "/api/channels/{channel}/stats",
            axum::routing::get(handle_get_channel_stats),
        )
        .route("/api/stats", axum::routing::get(handle_get_stats))
        .route("/api/v1/health", axum::routing::get(handle_get_health))
        .route("/api/v1/pending", axum::routing::get(handle_get_pending))
        .route(
            "/api/v1/messages",
            axum::routing::post(handle_post_messages),
        )
        .route(
            "/api/v1/messages/{id}",
            axum::routing::get(handle_get_message),
        )
        .route(
            "/api/v1/messages/{id}/response",
            axum::routing::post(handle_post_response),
        )
        .route(
            "/api/v1/tasks",
            axum::routing::post(handle_post_tasks).get(handle_get_tasks),
        )
        // literal-segment routes BEFORE parameterized {id} to prevent "ready"/"blocked" being
        // matched as UUIDs
        .route(
            "/api/v1/tasks/ready",
            axum::routing::get(handle_get_ready_tasks),
        )
        .route(
            "/api/v1/tasks/blocked",
            axum::routing::get(handle_get_blocked_tasks),
        )
        .route(
            "/api/v1/tasks/{id}",
            axum::routing::get(handle_get_task).put(handle_put_task),
        )
        .route(
            "/api/v1/tasks/{id}/dependencies",
            axum::routing::post(handle_post_task_dependencies).get(handle_get_task_dependencies),
        )
        .route(
            "/api/v1/tasks/{id}/dependencies/{dep_id}",
            axum::routing::delete(handle_delete_task_dependency),
        )
        .route(
            "/api/v1/tasks/{id}/graph",
            axum::routing::get(handle_get_task_graph),
        )
}

/// Handle POST /api/test
async fn handle_post_test() -> Json<serde_json::Value> {
    Json(serde_json::json!({"test": "ok"}))
}

/// Handle GET /api/channels
async fn handle_get_channels(
    State(state): State<AppState>,
) -> Result<Json<ChannelsResponse>, ApiError> {
    let channels = state.message_history.get_channels().await;

    let mut channel_infos = Vec::new();
    for channel_name in channels {
        let stats = state.message_history.get_channel_stats(&channel_name).await;
        let info = ChannelInfo {
            name: channel_name,
            message_count: stats.message_count,
            oldest_message: stats.oldest_message.map(|dt| dt.to_rfc3339()),
            newest_message: stats.newest_message.map(|dt| dt.to_rfc3339()),
        };
        channel_infos.push(info);
    }

    Ok(Json(ChannelsResponse {
        channels: channel_infos,
    }))
}

/// Handle GET /api/channels/:channel/messages
async fn handle_get_channel_messages(
    State(state): State<AppState>,
    Path(channel): Path<String>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<MessagesResponse>, ApiError> {
    let limit = query.limit.unwrap_or(100);
    let messages = state
        .message_history
        .get_messages(&channel, Some(limit))
        .await;

    let message_values: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|msg| serde_json::to_value(msg).unwrap_or_else(|_| serde_json::json!({})))
        .collect();

    Ok(Json(MessagesResponse {
        channel,
        total_count: message_values.len(),
        messages: message_values,
    }))
}

/// Handle GET /api/channels/:channel/stats
async fn handle_get_channel_stats(
    State(state): State<AppState>,
    Path(channel): Path<String>,
) -> Result<Json<StatsResponse>, ApiError> {
    let stats = state.message_history.get_channel_stats(&channel).await;

    Ok(Json(StatsResponse {
        channel,
        message_count: stats.message_count,
        oldest_message: stats.oldest_message.map(|dt| dt.to_rfc3339()),
        newest_message: stats.newest_message.map(|dt| dt.to_rfc3339()),
    }))
}

/// Handle GET /api/stats
async fn handle_get_stats(
    State(state): State<AppState>,
) -> Result<Json<crate::server::broadcast::BroadcastStats>, ApiError> {
    Ok(Json(state.broadcast_manager.get_stats().await))
}

/// Handle GET /api/v1/health
async fn handle_get_health(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, ApiError> {
    let broadcast_stats = state.broadcast_manager.get_stats().await;
    let queue_size = state
        .pending_prompt_registry
        .snapshot_pending(None)
        .await
        .len();

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        active_connections: broadcast_stats.total_viewers,
        queue_size,
        active_channels: broadcast_stats.active_channels,
    }))
}

/// Handle GET /api/v1/pending
async fn handle_get_pending(
    State(state): State<AppState>,
    Query(query): Query<PendingQuery>,
) -> Result<Json<PendingListResponse>, ApiError> {
    if let Some(ref ch) = query.channel {
        ailoop_core::channel::validation::validate_channel_name(ch)
            .map_err(|e| ApiError::ValidationError(e.to_string()))?;
    }

    let snapshots = state
        .pending_prompt_registry
        .snapshot_pending(query.channel.as_deref())
        .await;

    let items: Vec<PendingItemResponse> = snapshots
        .into_iter()
        .map(|s| {
            let kind = match s.prompt_type {
                crate::server::providers::PromptType::Decision => "decision",
                crate::server::providers::PromptType::Authorization => "authorize",
                crate::server::providers::PromptType::Navigation => "navigate",
            };
            PendingItemResponse {
                message_id: s.message_id,
                kind: kind.to_string(),
                channel: s.channel,
                position: s.position + 1,
                label: s.label,
            }
        })
        .collect();

    let total_count = items.len();
    Ok(Json(PendingListResponse { items, total_count }))
}

/// Handle POST /api/v1/messages
async fn handle_post_messages(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<Response, ApiError> {
    if state
        .is_shutting_down
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "server shutting down"})),
        )
            .into_response());
    }

    ailoop_core::channel::validation::validate_channel_name(&message.channel)
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    state
        .message_history
        .add_message(&message.channel, message.clone())
        .await;

    state.broadcast_manager.broadcast_message(&message).await;

    Ok((StatusCode::CREATED, Json(message)).into_response())
}

/// Handle GET /api/v1/messages/:id
async fn handle_get_message(
    State(state): State<AppState>,
    Path(message_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    match state.message_history.get_message_by_id(&message_id).await {
        Some(message) => Ok((StatusCode::OK, Json(message)).into_response()),
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Message not found",
                "message_id": message_id.to_string()
            })),
        )
            .into_response()),
    }
}

/// Handle POST /api/v1/messages/:id/response
async fn handle_post_response(
    State(state): State<AppState>,
    Path(message_id): Path<Uuid>,
    Json(response_request): Json<ResponseRequest>,
) -> Result<Response, ApiError> {
    let original_message = match state.message_history.get_message_by_id(&message_id).await {
        Some(msg) => msg,
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Original message not found",
                    "message_id": message_id.to_string()
                })),
            )
                .into_response());
        }
    };

    let answer = response_request.answer.clone();
    let response_type = response_request.response_type.clone();

    let response_content = ailoop_core::models::MessageContent::Response {
        answer: answer.clone(),
        response_type: response_type.clone(),
    };

    let response_message = ailoop_core::models::Message::response(
        original_message.channel.clone(),
        response_content,
        message_id,
    );

    state
        .message_history
        .add_message(&response_message.channel, response_message.clone())
        .await;

    state
        .broadcast_manager
        .broadcast_message(&response_message)
        .await;

    state
        .pending_prompt_registry
        .submit_reply_for_message(message_id, answer, response_type)
        .await;

    Ok((StatusCode::OK, Json(response_message)).into_response())
}

/// Handle POST /api/v1/tasks
async fn handle_post_tasks(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Response, ApiError> {
    let mut task = ailoop_core::models::Task::new(request.title, request.description);
    if let Some(assignee) = request.assignee {
        task = task.with_assignee(assignee);
    }
    if let Some(metadata) = request.metadata {
        task = task.with_metadata(metadata);
    }

    let created = state
        .task_storage
        .create_task(request.channel.clone(), task)
        .await
        .map_err(|e| ApiError::ValidationError(format!("Failed to create task: {}", e)))?;

    Ok((StatusCode::CREATED, Json(created)).into_response())
}

/// Handle GET /api/v1/tasks
async fn handle_get_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<TasksResponse>, ApiError> {
    let filter_state = query._state.and_then(|s| match s.as_str() {
        "pending" => Some(TaskState::Pending),
        "done" => Some(TaskState::Done),
        "abandoned" => Some(TaskState::Abandoned),
        _ => None,
    });

    let tasks = state
        .task_storage
        .list_tasks(&query.channel, filter_state)
        .await;

    Ok(Json(TasksResponse {
        channel: query.channel,
        total_count: tasks.len(),
        tasks,
    }))
}

/// Handle GET /api/v1/tasks/:id
async fn handle_get_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<TaskChannelQuery>,
) -> Result<Response, ApiError> {
    match state.task_storage.get_task(&query.channel, task_id).await {
        Some(task) => Ok((StatusCode::OK, Json(task)).into_response()),
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Task not found",
                "task_id": task_id.to_string()
            })),
        )
            .into_response()),
    }
}

/// Handle PUT /api/v1/tasks/:id
async fn handle_put_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<TaskChannelQuery>,
    Json(request): Json<UpdateTaskRequest>,
) -> Result<Response, ApiError> {
    match state
        .task_storage
        .update_task_state(&query.channel, task_id, request.state)
        .await
    {
        Ok(task) => Ok((StatusCode::OK, Json(task)).into_response()),
        Err(e) => Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string(),
                "task_id": task_id.to_string()
            })),
        )
            .into_response()),
    }
}

/// Handle POST /api/v1/tasks/:id/dependencies
async fn handle_post_task_dependencies(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<TaskChannelQuery>,
    Json(request): Json<AddDependencyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .task_storage
        .add_dependency(
            query.channel,
            task_id,
            request.parent_id,
            request.dependency_type,
        )
        .await
        .map_err(|e| ApiError::ValidationError(format!("Failed to add dependency: {}", e)))?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Handle DELETE /api/v1/tasks/:id/dependencies/:dep_id
async fn handle_delete_task_dependency(
    State(state): State<AppState>,
    Path((task_id, dep_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<TaskChannelQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .task_storage
        .remove_dependency(query.channel, task_id, dep_id)
        .await
        .map_err(|e| ApiError::ValidationError(format!("Failed to remove dependency: {}", e)))?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Handle GET /api/v1/tasks/ready
async fn handle_get_ready_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<TasksResponse>, ApiError> {
    let tasks = state.task_storage.get_ready_tasks(&query.channel).await;

    Ok(Json(TasksResponse {
        channel: query.channel,
        total_count: tasks.len(),
        tasks,
    }))
}

/// Handle GET /api/v1/tasks/blocked
async fn handle_get_blocked_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<TasksResponse>, ApiError> {
    let tasks = state.task_storage.get_blocked_tasks(&query.channel).await;

    Ok(Json(TasksResponse {
        channel: query.channel,
        total_count: tasks.len(),
        tasks,
    }))
}

/// Handle GET /api/v1/tasks/:id/dependencies
async fn handle_get_task_dependencies(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<TaskChannelQuery>,
) -> Result<Response, ApiError> {
    match state.task_storage.get_task(&query.channel, task_id).await {
        Some(task) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "task_id": task_id,
                "depends_on": task.depends_on,
                "blocking_for": task.blocking_for
            })),
        )
            .into_response()),
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Task not found",
                "task_id": task_id.to_string()
            })),
        )
            .into_response()),
    }
}

/// Handle GET /api/v1/tasks/:id/graph
async fn handle_get_task_graph(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<TaskChannelQuery>,
) -> Result<Response, ApiError> {
    match state
        .task_storage
        .get_dependency_graph(&query.channel, task_id)
        .await
    {
        Ok(graph) => Ok((StatusCode::OK, Json(graph)).into_response()),
        Err(e) => Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string(),
                "task_id": task_id.to_string()
            })),
        )
            .into_response()),
    }
}
