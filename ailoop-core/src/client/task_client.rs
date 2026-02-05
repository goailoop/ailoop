use crate::models::{DependencyType, Task, TaskState};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize)]
struct CreateTaskPayload {
    title: String,
    description: String,
    channel: String,
    assignee: Option<String>,
    metadata: Option<Value>,
}

#[derive(Serialize)]
struct UpdateTaskPayload {
    state: TaskState,
}

#[derive(Serialize)]
struct AddDependencyPayload {
    child_id: Uuid,
    parent_id: Uuid,
    dependency_type: DependencyType,
}

#[derive(Deserialize)]
struct TasksResponsePayload {
    tasks: Vec<Task>,
}

pub struct TaskClient {
    base_url: String,
    client: Client,
}

impl TaskClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base = base_url.into();
        let base = base.trim_end_matches('/').to_string();
        Self {
            base_url: base,
            client: Client::new(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }

    async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Server error {}: {}", status, body.trim()))
        }
    }

    pub async fn create_task(
        &self,
        title: &str,
        description: &str,
        channel: &str,
        assignee: Option<String>,
        metadata: Option<Value>,
    ) -> Result<Task> {
        let payload = CreateTaskPayload {
            title: title.to_string(),
            description: description.to_string(),
            channel: channel.to_string(),
            assignee,
            metadata,
        };

        let response = self
            .client
            .post(self.endpoint("api/v1/tasks"))
            .json(&payload)
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let task = response
            .json::<Task>()
            .await
            .context("Failed to parse task creation response")?;
        Ok(task)
    }

    pub async fn list_tasks(&self, channel: &str, state: Option<TaskState>) -> Result<Vec<Task>> {
        let mut params = vec![("channel", channel.to_string())];
        if let Some(state) = state {
            params.push(("state", state.to_string()));
        }

        let response = self
            .client
            .get(self.endpoint("api/v1/tasks"))
            .query(&params)
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let payload = response
            .json::<TasksResponsePayload>()
            .await
            .context("Failed to parse task list response")?;
        Ok(payload.tasks)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Task> {
        let task_uuid = Uuid::parse_str(task_id).context("Invalid task ID")?;
        let response = self
            .client
            .get(self.endpoint(&format!("api/v1/tasks/{}", task_uuid)))
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let task = response
            .json::<Task>()
            .await
            .context("Failed to parse task response")?;
        Ok(task)
    }

    pub async fn update_task_state(&self, task_id: &str, state: TaskState) -> Result<Task> {
        let task_uuid = Uuid::parse_str(task_id).context("Invalid task ID")?;
        let payload = UpdateTaskPayload { state };

        let response = self
            .client
            .put(self.endpoint(&format!("api/v1/tasks/{}", task_uuid)))
            .json(&payload)
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let task = response
            .json::<Task>()
            .await
            .context("Failed to parse task update response")?;
        Ok(task)
    }

    pub async fn add_dependency(
        &self,
        child_id: &str,
        parent_id: &str,
        dependency_type: DependencyType,
    ) -> Result<()> {
        let child_uuid = Uuid::parse_str(child_id).context("Invalid child task ID")?;
        let parent_uuid = Uuid::parse_str(parent_id).context("Invalid parent task ID")?;

        let payload = AddDependencyPayload {
            child_id: child_uuid,
            parent_id: parent_uuid,
            dependency_type,
        };

        let response = self
            .client
            .post(self.endpoint(&format!("api/v1/tasks/{}/dependencies", child_uuid)))
            .json(&payload)
            .send()
            .await?;

        Self::ensure_success(response).await.map(|_| ())
    }

    pub async fn remove_dependency(&self, child_id: &str, dependency_id: &str) -> Result<()> {
        let child_uuid = Uuid::parse_str(child_id).context("Invalid child task ID")?;
        let dependency_uuid = Uuid::parse_str(dependency_id).context("Invalid dependency ID")?;

        let response = self
            .client
            .delete(self.endpoint(&format!(
                "api/v1/tasks/{}/dependencies/{}",
                child_uuid, dependency_uuid
            )))
            .send()
            .await?;

        Self::ensure_success(response).await.map(|_| ())
    }

    pub async fn get_dependency_graph(&self, task_id: &str) -> Result<Value> {
        let task_uuid = Uuid::parse_str(task_id).context("Invalid task ID")?;
        let response = self
            .client
            .get(self.endpoint(&format!("api/v1/tasks/{}/graph", task_uuid)))
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let graph = response
            .json::<Value>()
            .await
            .context("Failed to parse dependency graph")?;
        Ok(graph)
    }

    pub async fn list_ready_tasks(&self, channel: &str) -> Result<Vec<Task>> {
        self.fetch_tasks_with_path("api/v1/tasks/ready", channel)
            .await
    }

    pub async fn list_blocked_tasks(&self, channel: &str) -> Result<Vec<Task>> {
        self.fetch_tasks_with_path("api/v1/tasks/blocked", channel)
            .await
    }

    async fn fetch_tasks_with_path(&self, path: &str, channel: &str) -> Result<Vec<Task>> {
        let response = self
            .client
            .get(self.endpoint(path))
            .query(&[("channel", channel)])
            .send()
            .await?;

        let response = Self::ensure_success(response).await?;
        let payload = response
            .json::<TasksResponsePayload>()
            .await
            .context("Failed to parse task list response")?;
        Ok(payload.tasks)
    }
}
