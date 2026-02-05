use super::task::{DepCommands, TaskCommands};

use anyhow::{bail, Result};
use serde_json::json;

use ailoop_core::client::task_client::TaskClient;
use ailoop_core::models::{DependencyType, TaskState};

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
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let task = client
        .create_task(&title, &description, &channel, None, None)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Task created:");
        println!("  ID: {}", task.id);
        println!("  Title: {}", task.title);
        println!("  State: {}", task.state);
        println!("  Created: {}", task.created_at);
    }

    Ok(())
}

async fn handle_task_list(
    channel: String,
    state: Option<String>,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let state_filter = match state {
        Some(state_value) => Some(parse_task_state(&state_value)?),
        None => None,
    };

    let tasks = client.list_tasks(&channel, state_filter).await?;

    if json {
        let payload = json!({
            "channel": channel,
            "tasks": tasks,
            "total_count": tasks.len()
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Tasks in channel '{}':", channel);
        if tasks.is_empty() {
            println!("  No tasks");
        } else {
            for task in tasks.iter() {
                println!(
                    "  - [{}] {} ({}){}",
                    task.state,
                    task.title,
                    task.id,
                    if task.blocked { " [BLOCKED]" } else { "" }
                );
            }
        }
    }

    Ok(())
}

async fn handle_task_show(
    task_id: String,
    _channel: String,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let task = client.get_task(&task_id).await?;

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

    Ok(())
}

async fn handle_task_update(
    task_id: String,
    state: String,
    _channel: String,
    server: String,
    json: bool,
) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let task_state = parse_task_state(&state)?;

    let task = client.update_task_state(&task_id, task_state).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Task updated:");
        println!("  ID: {}", task.id);
        println!("  New state: {}", task.state);
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
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let dep_type = match dependency_type.to_lowercase().as_str() {
        "blocks" => DependencyType::Blocks,
        "related" => DependencyType::Related,
        "parent" => DependencyType::Parent,
        _ => bail!("Invalid dependency type. Must be blocks, related, or parent"),
    };

    client
        .add_dependency(&child_id, &parent_id, dep_type)
        .await?;

    println!("Dependency added: {} depends on {}", child_id, parent_id);
    Ok(())
}

async fn handle_dep_remove(
    child_id: String,
    parent_id: String,
    _channel: String,
    server: String,
) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    client.remove_dependency(&child_id, &parent_id).await?;

    println!(
        "Dependency removed: {} no longer depends on {}",
        child_id, parent_id
    );
    Ok(())
}

async fn handle_dep_graph(task_id: String, _channel: String, server: String) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let graph = client.get_dependency_graph(&task_id).await?;

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
                let title = parent["title"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid parent title in graph"))?;
                let state = parent["state"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid parent state in graph"))?;
                println!("  - [{}] {}", state, title);
            }
        }
    }

    if let Some(children) = graph["children"].as_array() {
        if !children.is_empty() {
            println!("\nChildren:");
            for child in children {
                let title = child["title"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid child title in graph"))?;
                let state = child["state"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid child state in graph"))?;
                println!("  - [{}] {}", state, title);
            }
        }
    }

    if graph["parents"].as_array().is_none_or(|p| p.is_empty())
        && graph["children"].as_array().is_none_or(|c| c.is_empty())
    {
        println!("\nNo dependencies");
    }

    Ok(())
}

async fn handle_task_ready(channel: String, server: String, json: bool) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let tasks = client.list_ready_tasks(&channel).await?;

    if json {
        let payload = json!({
            "channel": channel,
            "tasks": tasks,
            "total_count": tasks.len()
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Ready tasks in channel '{}':", channel);
        if tasks.is_empty() {
            println!("  No ready tasks");
        } else {
            for task in tasks.iter() {
                println!("  - {} ({})", task.title, task.id);
            }
        }
    }

    Ok(())
}

async fn handle_task_blocked(channel: String, server: String, json: bool) -> Result<()> {
    let server_url = resolve_server_url(server)?;
    let client = TaskClient::new(&server_url);

    let tasks = client.list_blocked_tasks(&channel).await?;

    if json {
        let payload = json!({
            "channel": channel,
            "tasks": tasks,
            "total_count": tasks.len()
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Blocked tasks in channel '{}':", channel);
        if tasks.is_empty() {
            println!("  No blocked tasks");
        } else {
            for task in tasks.iter() {
                println!("  - {} ({})", task.title, task.id);
                for dep in task.depends_on.iter() {
                    println!("    ⬅ Depends on: {}", dep);
                }
            }
        }
    }

    Ok(())
}

fn resolve_server_url(server: String) -> Result<String> {
    if server.is_empty() {
        get_server_url()
    } else {
        Ok(server)
    }
}

fn parse_task_state(value: &str) -> Result<TaskState> {
    match value.to_lowercase().as_str() {
        "pending" => Ok(TaskState::Pending),
        "done" => Ok(TaskState::Done),
        "abandoned" => Ok(TaskState::Abandoned),
        other => bail!(
            "Invalid state: {}. Must be pending, done, or abandoned",
            other
        ),
    }
}

fn get_server_url() -> Result<String> {
    Ok("http://127.0.0.1:8080".to_string())
}
