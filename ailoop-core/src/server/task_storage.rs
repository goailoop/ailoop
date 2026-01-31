use crate::models::{DependencyType, Task, TaskState};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TaskDependency {
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub dependency_type: DependencyType,
    pub created_at: DateTime<Utc>,
}

pub struct TaskStorage {
    tasks: DashMap<(String, Uuid), Task>,
    dependencies: DashMap<(String, Uuid), Vec<TaskDependency>>,
    blocked_cache: DashMap<(String, Uuid), bool>,
}

impl TaskStorage {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            dependencies: DashMap::new(),
            blocked_cache: DashMap::new(),
        }
    }

    pub async fn add_dependency(
        &self,
        channel: String,
        child_id: Uuid,
        parent_id: Uuid,
        dependency_type: DependencyType,
    ) -> Result<()> {
        let dep_key = (channel.clone(), child_id);

        if !self.tasks.contains_key(&(channel.clone(), child_id)) {
            return Err(anyhow!("Child task {} not found", child_id));
        }

        if !self.tasks.contains_key(&(channel.clone(), parent_id)) {
            return Err(anyhow!("Parent task {} not found", parent_id));
        }

        let existing_deps = self
            .dependencies
            .get(&dep_key)
            .map(|v| v.clone())
            .unwrap_or_default();

        for dep in existing_deps.iter() {
            if dep.child_id == child_id && dep.parent_id == parent_id {
                return Err(anyhow!("Dependency already exists"));
            }
        }

        if self.check_circular_dependency(&channel, child_id, parent_id) {
            return Err(anyhow!(
                "Circular dependency detected: cannot add dependency from {} to {}",
                child_id,
                parent_id
            ));
        }

        let new_dep = TaskDependency {
            parent_id,
            child_id,
            dependency_type,
            created_at: Utc::now(),
        };

        self.dependencies.insert(dep_key, vec![new_dep]);

        let mut child_task = self
            .tasks
            .get(&(channel.clone(), child_id))
            .unwrap()
            .clone();
        child_task.depends_on.push(parent_id);
        self.update_blocked_status(&channel, &mut child_task)
            .await?;
        self.tasks.insert((channel.clone(), child_id), child_task);

        Ok(())
    }

    pub async fn remove_dependency(
        &self,
        channel: String,
        child_id: Uuid,
        parent_id: Uuid,
    ) -> Result<()> {
        let dep_key = (channel.clone(), child_id);

        if let Some(mut deps) = self.dependencies.get_mut(&dep_key) {
            deps.retain(|dep| dep.child_id != child_id || dep.parent_id != parent_id);
        }

        if let Some(mut child_task) = self
            .tasks
            .get(&(channel.clone(), child_id))
            .map(|t| t.clone())
        {
            self.update_blocked_status(&channel, &mut child_task)
                .await?;
            self.tasks.insert((channel.clone(), child_id), child_task);
        }

        Ok(())
    }

    pub async fn get_ready_tasks(&self, channel: &str) -> Vec<Task> {
        let mut tasks: Vec<Task> = self
            .tasks
            .iter()
            .filter(|entry| {
                let (ch, task_id) = entry.key();
                ch == channel && !task_id.is_nil() && !entry.value().blocked
            })
            .map(|entry| entry.value().clone())
            .collect();
        tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        tasks
    }

    pub async fn get_blocked_tasks(&self, channel: &str) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|entry| {
                let (ch, task_id) = entry.key();
                ch == channel && !task_id.is_nil() && entry.value().blocked
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn get_dependency_graph(
        &self,
        channel: &str,
        task_id: Uuid,
    ) -> Result<DependencyGraph> {
        let task_key = (channel.to_string(), task_id);

        if !self.tasks.contains_key(&task_key) {
            return Err(anyhow!("Task {} not found", task_id));
        }

        let task = self.tasks.get(&task_key).unwrap().clone();

        let mut parents = Vec::new();
        if let Some(deps) = self.dependencies.get(&(channel.to_string(), task_id)) {
            for dep in deps.iter() {
                if let Some(parent_task) = self.tasks.get(&(channel.to_string(), dep.parent_id)) {
                    parents.push(parent_task.clone());
                }
            }
        }

        let mut children = Vec::new();
        for entry in self.tasks.iter() {
            let (ch, id) = entry.key();
            if ch == channel && !id.is_nil() {
                if let Some(deps) = self.dependencies.get(&(ch.to_string(), *id)) {
                    for dep in deps.iter() {
                        if dep.parent_id == task_id {
                            if let Some(child_task) =
                                self.tasks.get(&(ch.to_string(), dep.child_id))
                            {
                                children.push(child_task.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(DependencyGraph {
            task,
            parents,
            children,
        })
    }

    pub async fn create_task(&self, channel: String, mut task: Task) -> Result<Task> {
        let key = (channel.clone(), task.id);
        if self.tasks.contains_key(&key) {
            return Err(anyhow!(
                "Task with ID {} already exists in channel {}",
                task.id,
                channel
            ));
        }

        self.update_blocked_status(&channel, &mut task).await?;

        self.tasks.insert(key, task.clone());

        Ok(task)
    }

    pub async fn get_task(&self, channel: &str, task_id: Uuid) -> Option<Task> {
        self.tasks
            .get(&(channel.to_string(), task_id))
            .map(|t| t.clone())
    }

    pub async fn list_tasks(&self, channel: &str, state: Option<TaskState>) -> Vec<Task> {
        let mut tasks = Vec::new();
        for entry in self.tasks.iter() {
            let (ch, _id) = entry.key();
            let task = entry.value();
            if ch == channel && (state.is_none() || Some(&task.state) == state.as_ref()) {
                tasks.push(task.clone());
            }
        }
        tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        tasks
    }

    pub async fn update_task_state(
        &self,
        channel: &str,
        task_id: Uuid,
        state: TaskState,
    ) -> Result<Task> {
        let key = (channel.to_string(), task_id);
        let mut task = self
            .tasks
            .get(&key)
            .ok_or_else(|| anyhow!("Task {} not found in channel {}", task_id, channel))?
            .clone();

        task.state = state.clone();
        task.updated_at = Utc::now();

        self.update_blocked_status(channel, &mut task).await?;

        self.tasks.insert(key.clone(), task.clone());

        if let Some(dependencies) = self.dependencies.get(&(channel.to_string(), task_id)) {
            for dep in dependencies.iter() {
                let child_key = (channel.to_string(), dep.child_id);
                if let Some(mut child_task) = self.tasks.get(&child_key).map(|t| t.clone()) {
                    self.update_blocked_status(channel, &mut child_task).await?;
                    self.tasks.insert(child_key.clone(), child_task);
                }
            }
        }

        let final_task = self.tasks.get(&key).unwrap().clone();
        Ok(final_task)
    }

    async fn update_blocked_status(&self, channel: &str, task: &mut Task) -> Result<()> {
        let mut blocked = false;
        for parent_id in &task.depends_on {
            if let Some(parent) = self.get_task(channel, *parent_id).await {
                if parent.state != TaskState::Done {
                    blocked = true;
                    break;
                }
            }
        }

        task.blocked = blocked;
        task.updated_at = Utc::now();

        self.blocked_cache
            .insert((channel.to_string(), task.id), blocked);

        Ok(())
    }

    fn check_circular_dependency(&self, channel: &str, child_id: Uuid, parent_id: Uuid) -> bool {
        let mut visited = HashSet::new();
        self.has_path(channel, parent_id, child_id, &mut visited)
    }

    fn has_path(&self, channel: &str, from: Uuid, to: Uuid, visited: &mut HashSet<Uuid>) -> bool {
        if from == to {
            return true;
        }

        if visited.contains(&from) {
            return false;
        }

        visited.insert(from);

        if let Some(dependencies) = self.dependencies.get(&(channel.to_string(), from)) {
            for dep in dependencies.iter() {
                if self.has_path(channel, dep.parent_id, to, visited) {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for TaskStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    pub task: Task,
    pub parents: Vec<Task>,
    pub children: Vec<Task>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_task() {
        let storage = TaskStorage::new();
        let task = Task::new("Test Task".to_string(), "Test Description".to_string());

        let result = storage
            .create_task("public".to_string(), task.clone())
            .await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.title, "Test Task");
        assert_eq!(created.state, TaskState::Pending);
    }

    #[tokio::test]
    async fn test_get_task() {
        let storage = TaskStorage::new();
        let task = Task::new("Test Task".to_string(), "Test Description".to_string());
        let created = storage
            .create_task("public".to_string(), task.clone())
            .await
            .unwrap();

        let retrieved = storage.get_task("public", created.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Task");
    }

    #[tokio::test]
    async fn test_add_dependency() {
        let storage = TaskStorage::new();
        let parent = Task::new("Parent Task".to_string(), "Parent Description".to_string());
        let child = Task::new("Child Task".to_string(), "Child Description".to_string());

        let parent = storage
            .create_task("public".to_string(), parent)
            .await
            .unwrap();
        let child = storage
            .create_task("public".to_string(), child)
            .await
            .unwrap();

        let result = storage
            .add_dependency(
                "public".to_string(),
                child.id,
                parent.id,
                DependencyType::Blocks,
            )
            .await;
        assert!(result.is_ok());

        let retrieved = storage.get_task("public", child.id).await.unwrap();
        assert!(retrieved.blocked);
    }

    #[tokio::test]
    async fn test_circular_dependency_prevention() {
        let storage = TaskStorage::new();
        let task_a = Task::new("Task A".to_string(), "A".to_string());
        let task_b = Task::new("Task B".to_string(), "B".to_string());
        let task_c = Task::new("Task C".to_string(), "C".to_string());

        let task_a = storage
            .create_task("public".to_string(), task_a)
            .await
            .unwrap();
        let task_b = storage
            .create_task("public".to_string(), task_b)
            .await
            .unwrap();
        let task_c = storage
            .create_task("public".to_string(), task_c)
            .await
            .unwrap();

        storage
            .add_dependency(
                "public".to_string(),
                task_b.id,
                task_a.id,
                DependencyType::Blocks,
            )
            .await
            .unwrap();
        storage
            .add_dependency(
                "public".to_string(),
                task_c.id,
                task_b.id,
                DependencyType::Blocks,
            )
            .await
            .unwrap();

        let result = storage
            .add_dependency(
                "public".to_string(),
                task_a.id,
                task_c.id,
                DependencyType::Blocks,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ready_tasks() {
        let storage = TaskStorage::new();
        let task1 = Task::new("Ready Task".to_string(), "Ready".to_string());
        let task2 = Task::new("Blocked Task".to_string(), "Blocked".to_string());

        let task1 = storage
            .create_task("public".to_string(), task1)
            .await
            .unwrap();
        let task2 = storage
            .create_task("public".to_string(), task2)
            .await
            .unwrap();

        let ready = storage.get_ready_tasks("public").await;
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].id, task1.id);
        assert_eq!(ready[1].id, task2.id);
    }
}
