use super::task::{DepCommands, TaskCommands};

use anyhow::Result;
use reqwest::Client;
use serde_json::json;

use ailoop_core::models::{DependencyType, Task, TaskState};

pub async fn handle_task_commands(command: TaskCommands) -> Result<()> {
    match command {
        TaskCommands::Create {
            title,
            description,
            channel,
            server,
            json,
        } => {
            handle_task_create(title, description, channel, server, json).await?;
        }
        TaskCommands::List {
            channel,
            state,
            server,
            json,
        } => {
            handle_task_list(channel, state, server, json).await?;
        }
        TaskCommands::Show {
            task_id,
            channel,
            server,
            json,
        } => {
            handle_task_show(task_id, channel, server, json).await?;
        }
        TaskCommands::Update {
            task_id,
            state,
            channel,
            server,
            json,
        } => {
            handle_task_update(task_id, state, channel, server, json).await?;
        }
        TaskCommands::Dep { command } => {
            handle_task_dep(command).await?;
        }
        TaskCommands::Ready {
            channel,
            server,
            json,
        } => {
            handle_task_ready(channel, server, json).await?;
        }
        TaskCommands::Blocked {
            channel,
            server,
            json,
        } => {
            handle_task_blocked(channel, server, json).await?;
        }
    }
    Ok(())
}

async fn handle_task_create(
    title: String,
    description: String,
    channel: String,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let request_body = json!({
        "title": title,
        "description": description,
        "channel": channel
    });

    let response = client
        .post(format!("{}/api/v1/tasks", server_url))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let task: Task = response.json().await?;
        if json {
            println!("{}", serde_json::to_string_pretty(&task)?);
        } else {
            println!("Task created:");
            println!("  ID: {}", task.id);
            println!("  Title: {}", task.title);
            println!("  State: {}", task.state);
            println!("  Created: {}", task.created_at);
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to create task: {}", error);
    }

    Ok(())
}

async fn handle_task_list(
    channel: String,
    state: Option<String>,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let mut url = format!("{}/api/v1/tasks?channel={}", server_url, channel);
    if let Some(s) = state {
        url.push_str(&format!("&state={}", s));
    }

    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        let tasks = data["tasks"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in response"))?;

        if json {
            println!("{}", serde_json::to_string_pretty(&data)?);
        } else {
            println!("Tasks in channel '{}':", channel);
            for task in tasks {
                let id = task["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid task_id in response"))?;
                let title = task["title"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid title in task"))?;
                let task_state = task["state"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid state in task"))?;
                let blocked = task["blocked"]
                    .as_bool()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid blocked status in task"))?;
                println!(
                    "  - [{}] {} ({}){}",
                    task_state,
                    title,
                    id,
                    if blocked { " [BLOCKED]" } else { "" }
                );
            }
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to list tasks: {}", error);
    }

    Ok(())
}

async fn handle_task_show(
    task_id: String,
    _channel: String,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let response = client
        .get(format!("{}/api/v1/tasks/{}", server_url, task_id))
        .send()
        .await?;

    if response.status().is_success() {
        let task: Task = response.json().await?;
        if json {
            println!("{}", serde_json::to_string_pretty(&task)?);
        } else {
            println!("Task Details:");
            println!("  ID: {}", task.id);
            println!("  Title: {}", task.title);
            println!("  Description: {}", task.description);
            println!("  State: {}", task.state);
            println!("  Blocked: {}", task.blocked);
            println!("  Created: {}", task.created_at);
            println!("  Updated: {}", task.updated_at);
            if let Some(assignee) = &task.assignee {
                println!("  Assignee: {}", assignee);
            }
            if !task.depends_on.is_empty() {
                println!("  Depends on: {:?}", task.depends_on);
            }
            if !task.blocking_for.is_empty() {
                println!("  Blocking: {:?}", task.blocking_for);
            }
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to get task: {}", error);
    }

    Ok(())
}

async fn handle_task_update(
    task_id: String,
    state: String,
    _channel: String,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let task_state = match state.to_lowercase().as_str() {
        "pending" => TaskState::Pending,
        "done" => TaskState::Done,
        "abandoned" => TaskState::Abandoned,
        _ => anyhow::bail!(
            "Invalid state: {}. Must be pending, done, or abandoned",
            state
        ),
    };

    let client = Client::new();
    let request_body = json!({ "state": task_state });

    let response = client
        .put(format!("{}/api/v1/tasks/{}", server_url, task_id))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let task: Task = response.json().await?;
        if json {
            println!("{}", serde_json::to_string_pretty(&task)?);
        } else {
            println!("Task updated:");
            println!("  ID: {}", task.id);
            println!("  New state: {}", task.state);
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to update task: {}", error);
    }

    Ok(())
}

async fn handle_task_dep(command: DepCommands) -> Result<()> {
    match command {
        DepCommands::Add {
            child_id,
            parent_id,
            dependency_type,
            channel,
            server,
        } => {
            handle_dep_add(child_id, parent_id, dependency_type, channel, server).await?;
        }
        DepCommands::Remove {
            child_id,
            parent_id,
            channel,
            server,
        } => {
            handle_dep_remove(child_id, parent_id, channel, server).await?;
        }
        DepCommands::Graph {
            task_id,
            channel,
            server,
        } => {
            handle_dep_graph(task_id, channel, server).await?;
        }
    }
    Ok(())
}

async fn handle_dep_add(
    child_id: String,
    parent_id: String,
    dependency_type: String,
    _channel: String,
    server: String,
) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let dep_type = match dependency_type.to_lowercase().as_str() {
        "blocks" => DependencyType::Blocks,
        "related" => DependencyType::Related,
        "parent" => DependencyType::Parent,
        _ => anyhow::bail!("Invalid dependency type. Must be blocks, related, or parent"),
    };

    let client = Client::new();
    let request_body = json!({
        "child_id": child_id,
        "parent_id": parent_id,
        "dependency_type": dep_type
    });

    let response = client
        .post(format!(
            "{}/api/v1/tasks/{}/dependencies",
            server_url, child_id
        ))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Dependency added: {} depends on {}", child_id, parent_id);
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to add dependency: {}", error);
    }

    Ok(())
}

async fn handle_dep_remove(
    child_id: String,
    parent_id: String,
    _channel: String,
    server: String,
) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let response = client
        .delete(format!(
            "{}/api/v1/tasks/{}/dependencies/{}",
            server_url, child_id, parent_id
        ))
        .send()
        .await?;

    if response.status().is_success() {
        println!(
            "Dependency removed: {} no longer depends on {}",
            child_id, parent_id
        );
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to remove dependency: {}", error);
    }

    Ok(())
}

async fn handle_dep_graph(task_id: String, _channel: String, server: String) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let response = client
        .get(format!("{}/api/v1/tasks/{}/graph", server_url, task_id))
        .send()
        .await?;

    if response.status().is_success() {
        let graph: serde_json::Value = response.json().await?;
        println!("Dependency graph for task {}:", task_id);
        println!(
            "\nTask: {}",
            graph["task"]["title"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing or invalid task title in graph"))?
        );

        if let Some(parents) = graph["parents"].as_array() {
            if !parents.is_empty() {
                println!("\nParents:");
                for parent in parents {
                    let title = parent["title"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid parent title in graph")
                    })?;
                    let state = parent["state"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid parent state in graph")
                    })?;
                    println!("  - [{}] {}", state, title);
                }
            }
        }

        if let Some(children) = graph["children"].as_array() {
            if !children.is_empty() {
                println!("\nChildren:");
                for child in children {
                    let title = child["title"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid child title in graph")
                    })?;
                    let state = child["state"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid child state in graph")
                    })?;
                    println!("  - [{}] {}", state, title);
                }
            }
        }

        if graph["parents"].as_array().is_none_or(|p| p.is_empty())
            && graph["children"].as_array().is_none_or(|c| c.is_empty())
        {
            println!("\nNo dependencies");
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to get dependency graph: {}", error);
    }

    Ok(())
}

async fn handle_task_ready(channel: String, server: String, json: bool) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let url = format!("{}/api/v1/tasks/ready?channel={}", server_url, channel);
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        let tasks = data["tasks"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in ready response"))?;

        if json {
            println!("{}", serde_json::to_string_pretty(&data)?);
        } else {
            println!("Ready tasks in channel '{}':", channel);
            if tasks.is_empty() {
                println!("  No ready tasks");
            } else {
                for task in tasks {
                    let id = task["id"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid task_id in ready response")
                    })?;
                    let title = task["title"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing or invalid title in ready task"))?;
                    println!("  - {} ({})", title, id);
                }
            }
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to get ready tasks: {}", error);
    }

    Ok(())
}

async fn handle_task_blocked(channel: String, server: String, json: bool) -> Result<()> {
    let server_url = if server.is_empty() {
        get_server_url()?
    } else {
        server
    };

    let client = Client::new();
    let url = format!("{}/api/v1/tasks/blocked?channel={}", server_url, channel);
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        let tasks = data["tasks"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in blocked response"))?;

        if json {
            println!("{}", serde_json::to_string_pretty(&data)?);
        } else {
            println!("Blocked tasks in channel '{}':", channel);
            if tasks.is_empty() {
                println!("  No blocked tasks");
            } else {
                for task in tasks {
                    let id = task["id"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid task_id in blocked response")
                    })?;
                    let title = task["title"].as_str().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid title in blocked task")
                    })?;
                    let depends_on = task["depends_on"].as_array().ok_or_else(|| {
                        anyhow::anyhow!("Missing or invalid depends_on array in blocked task")
                    })?;
                    println!("  - {} ({})", title, id);
                    for dep in depends_on {
                        println!("    ⬅ Depends on: {}", dep);
                    }
                }
            }
        }
    } else {
        let error: serde_json::Value = response.json().await?;
        anyhow::bail!("Failed to get blocked tasks: {}", error);
    }

    Ok(())
}

fn get_server_url() -> Result<String> {
    Ok("http://127.0.0.1:8080".to_string())
}
