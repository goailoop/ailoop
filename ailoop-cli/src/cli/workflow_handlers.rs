//! Workflow command handlers

use ailoop_core::workflow::{BashExecutor, WorkflowOrchestrator, WorkflowPersistence};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Get default workflow store path
fn get_workflow_store_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".ailoop").join("workflow_store.json")
}

/// Create workflow orchestrator instance
fn create_orchestrator() -> Result<WorkflowOrchestrator> {
    let store_path = get_workflow_store_path();
    let persistence = Arc::new(
        WorkflowPersistence::new(&store_path)
            .context("Failed to initialize workflow persistence")?,
    );
    let executor = Arc::new(BashExecutor::new());

    Ok(WorkflowOrchestrator::new(persistence, executor))
}

/// Handle workflow start command
pub async fn handle_workflow_start(
    workflow_name: String,
    initiator: String,
    json: bool,
) -> Result<()> {
    let orchestrator = create_orchestrator()?;

    // Check if workflow exists
    if orchestrator
        .get_workflow_definition(&workflow_name)
        .is_none()
    {
        return Err(anyhow::anyhow!(
            "Workflow '{}' not found. Use 'ailoop workflow list' to see available workflows.",
            workflow_name
        ));
    }

    // Start workflow
    let execution_id = orchestrator
        .start_workflow(&workflow_name, initiator.clone())
        .await
        .context("Failed to start workflow")?;

    if json {
        let output = serde_json::json!({
            "execution_id": execution_id.to_string(),
            "workflow_name": workflow_name,
            "initiator": initiator,
            "status": "started"
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✅ Started workflow '{}'", workflow_name);
        println!("   Execution ID: {}", execution_id);
        println!("   Initiator: {}", initiator);
        println!();
        println!(
            "Use 'ailoop workflow status {}' to check progress",
            execution_id
        );
    }

    Ok(())
}

/// Handle workflow status command
pub async fn handle_workflow_status(execution_id: String, json: bool) -> Result<()> {
    let store_path = get_workflow_store_path();
    let persistence = WorkflowPersistence::new(&store_path)
        .context("Failed to initialize workflow persistence")?;

    // Parse execution ID
    let exec_uuid = uuid::Uuid::parse_str(&execution_id)
        .context("Invalid execution ID format (expected UUID)")?;

    // Get execution status
    let execution = persistence
        .get_execution(exec_uuid)
        .ok_or_else(|| anyhow::anyhow!("Execution '{}' not found", execution_id))?;

    if json {
        let output = serde_json::json!({
            "execution_id": execution.id.to_string(),
            "workflow_name": execution.workflow_name,
            "current_state": execution.current_state,
            "status": format!("{:?}", execution.status),
            "started_at": execution.started_at.to_rfc3339(),
            "completed_at": execution.completed_at.map(|t| t.to_rfc3339()),
            "initiator": execution.initiator
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Workflow Execution Status");
        println!("========================");
        println!("Execution ID:   {}", execution.id);
        println!("Workflow:       {}", execution.workflow_name);
        println!("Current State:  {}", execution.current_state);
        println!("Status:         {:?}", execution.status);
        println!(
            "Started At:     {}",
            execution.started_at.format("%Y-%m-%d %H:%M:%S")
        );
        if let Some(completed_at) = execution.completed_at {
            println!(
                "Completed At:   {}",
                completed_at.format("%Y-%m-%d %H:%M:%S")
            );
            let duration = (completed_at - execution.started_at).num_seconds();
            println!("Duration:       {} seconds", duration);
        }
        println!("Initiator:      {}", execution.initiator);

        // Get and display transitions
        let transitions = persistence.get_transitions(exec_uuid);
        if !transitions.is_empty() {
            println!();
            println!("State Transitions:");
            println!("------------------");
            for transition in transitions {
                let from = transition.from_state.as_deref().unwrap_or("<initial>");
                println!(
                    "  {} -> {} ({:?}) at {}",
                    from,
                    transition.to_state,
                    transition.transition_type,
                    transition.timestamp.format("%H:%M:%S")
                );
            }
        }
    }

    Ok(())
}

/// Handle workflow list command
pub async fn handle_workflow_list(json: bool) -> Result<()> {
    let orchestrator = create_orchestrator()?;

    let workflows = orchestrator.list_workflows();

    if json {
        let output = serde_json::json!({
            "workflows": workflows,
            "count": workflows.len()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if workflows.is_empty() {
        println!("No workflows registered.");
        println!();
        println!("Register workflows by loading YAML definitions.");
    } else {
        println!("Available Workflows:");
        println!("===================");
        for workflow in workflows {
            if let Some(def) = orchestrator.get_workflow_definition(&workflow) {
                println!("  • {}", workflow);
                if let Some(desc) = def.description {
                    println!("    {}", desc);
                }
                println!(
                    "    Initial state: {}, Terminal states: {}",
                    def.initial_state,
                    def.terminal_states.join(", ")
                );
            } else {
                println!("  • {}", workflow);
            }
        }
    }

    Ok(())
}

/// Handle workflow history command
pub async fn handle_workflow_history(workflow: Option<String>, json: bool) -> Result<()> {
    let store_path = get_workflow_store_path();
    let persistence = WorkflowPersistence::new(&store_path)
        .context("Failed to initialize workflow persistence")?;

    // Get metrics
    let metrics = persistence.query_metrics(workflow.as_deref());

    if json {
        let output = serde_json::json!({
            "workflow": workflow.as_deref().unwrap_or("all"),
            "execution_count": metrics.execution_count,
            "success_count": metrics.success_count,
            "failure_count": metrics.failure_count,
            "avg_duration_ms": metrics.avg_duration_ms
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let workflow_name = workflow.as_deref().unwrap_or("All Workflows");
        println!("Workflow History: {}", workflow_name);
        println!("==================");
        println!("Total Executions:   {}", metrics.execution_count);
        println!("Successful:         {}", metrics.success_count);
        println!("Failed:             {}", metrics.failure_count);
        if metrics.execution_count > 0 {
            let success_rate =
                (metrics.success_count as f64 / metrics.execution_count as f64) * 100.0;
            println!("Success Rate:       {:.1}%", success_rate);
            println!(
                "Avg Duration:       {:.2}s",
                metrics.avg_duration_ms as f64 / 1000.0
            );
        }
    }

    Ok(())
}

/// Handle workflow approve command
pub async fn handle_workflow_approve(
    approval_id: String,
    operator: String,
    json: bool,
) -> Result<()> {
    let orchestrator = create_orchestrator()?;
    let approval_manager = orchestrator.approval_manager();

    // Parse approval ID
    let approval_uuid = uuid::Uuid::parse_str(&approval_id)
        .context("Invalid approval ID format (expected UUID)")?;

    // Check if approval exists
    let approval = approval_manager
        .get_approval_request(approval_uuid)
        .ok_or_else(|| anyhow::anyhow!("Approval request '{}' not found", approval_id))?;

    if approval.status != ailoop_core::models::workflow::ApprovalStatus::Pending {
        return Err(anyhow::anyhow!(
            "Approval request is not pending (current status: {:?})",
            approval.status
        ));
    }

    // Send approval
    approval_manager
        .respond_approval(
            approval_uuid,
            ailoop_core::workflow::ApprovalResponse::Approved,
            Some(operator.clone()),
        )
        .await
        .context("Failed to send approval")?;

    if json {
        let output = serde_json::json!({
            "approval_id": approval_id,
            "status": "approved",
            "operator": operator
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✅ Approval granted");
        println!("   Approval ID: {}", approval_id);
        println!("   Operator: {}", operator);
        println!("   Action: {}", approval.action_description);
    }

    Ok(())
}

/// Handle workflow deny command
pub async fn handle_workflow_deny(approval_id: String, operator: String, json: bool) -> Result<()> {
    let orchestrator = create_orchestrator()?;
    let approval_manager = orchestrator.approval_manager();

    // Parse approval ID
    let approval_uuid = uuid::Uuid::parse_str(&approval_id)
        .context("Invalid approval ID format (expected UUID)")?;

    // Check if approval exists
    let approval = approval_manager
        .get_approval_request(approval_uuid)
        .ok_or_else(|| anyhow::anyhow!("Approval request '{}' not found", approval_id))?;

    if approval.status != ailoop_core::models::workflow::ApprovalStatus::Pending {
        return Err(anyhow::anyhow!(
            "Approval request is not pending (current status: {:?})",
            approval.status
        ));
    }

    // Send denial
    approval_manager
        .respond_approval(
            approval_uuid,
            ailoop_core::workflow::ApprovalResponse::Denied,
            Some(operator.clone()),
        )
        .await
        .context("Failed to send denial")?;

    if json {
        let output = serde_json::json!({
            "approval_id": approval_id,
            "status": "denied",
            "operator": operator
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("🚫 Approval denied");
        println!("   Approval ID: {}", approval_id);
        println!("   Operator: {}", operator);
        println!("   Action: {}", approval.action_description);
    }

    Ok(())
}

/// Handle workflow list-approvals command
pub async fn handle_workflow_list_approvals(execution: Option<String>, json: bool) -> Result<()> {
    let store_path = get_workflow_store_path();
    let persistence = WorkflowPersistence::new(&store_path)
        .context("Failed to initialize workflow persistence")?;

    // Get pending approvals
    let approvals = if let Some(exec_id_str) = execution {
        let exec_uuid = uuid::Uuid::parse_str(&exec_id_str)
            .context("Invalid execution ID format (expected UUID)")?;
        persistence.get_pending_approvals(exec_uuid)
    } else {
        // TODO: Implement list_all_pending in persistence
        vec![]
    };

    if json {
        let output = serde_json::json!({
            "approvals": approvals.iter().map(|a| serde_json::json!({
                "approval_id": a.id.to_string(),
                "execution_id": a.execution_id.to_string(),
                "state_name": a.state_name,
                "action_description": a.action_description,
                "status": format!("{:?}", a.status),
                "requested_at": a.requested_at.to_rfc3339(),
                "timeout_seconds": a.timeout_seconds
            })).collect::<Vec<_>>(),
            "count": approvals.len()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if approvals.is_empty() {
        println!("No pending approval requests.");
    } else {
        println!("Pending Approval Requests:");
        println!("==========================");
        for approval in approvals {
            println!();
            println!("Approval ID:  {}", approval.id);
            println!("Execution ID: {}", approval.execution_id);
            println!("State:        {}", approval.state_name);
            println!("Action:       {}", approval.action_description);
            println!(
                "Requested:    {}",
                approval.requested_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!("Timeout:      {}s", approval.timeout_seconds);
            println!();
            println!("To approve: ailoop workflow approve {}", approval.id);
            println!("To deny:    ailoop workflow deny {}", approval.id);
            println!("---");
        }
    }

    Ok(())
}
/// Handle workflow logs command
pub async fn handle_workflow_logs(
    execution_id: String,
    state: Option<String>,
    limit: usize,
    offset: usize,
    follow: bool,
    json: bool,
) -> Result<()> {
    let execution_id =
        uuid::Uuid::parse_str(&execution_id).context("Invalid execution ID format")?;

    let store_path = get_workflow_store_path();
    let persistence = WorkflowPersistence::new(&store_path)
        .context("Failed to initialize workflow persistence")?;

    // Query output from persistence
    let output_chunks = persistence.query_output(execution_id, state.as_deref(), offset, limit);

    if follow {
        println!("Following output is not yet implemented in persistence layer.");
        println!("Showing recent output:");
        println!();
    }

    if json {
        // JSON output
        let json_output =
            serde_json::to_string_pretty(&output_chunks).context("Failed to serialize output")?;
        println!("{}", json_output);
    } else {
        // Human-readable output
        if output_chunks.is_empty() {
            println!("No output available for execution {}", execution_id);
        } else {
            println!("Output for execution {}:", execution_id);
            println!(
                "Showing {} chunks (offset: {})",
                output_chunks.len(),
                offset
            );
            println!();

            for chunk in &output_chunks {
                let output_type = match chunk.output_type {
                    ailoop_core::models::workflow::OutputType::Stdout => "stdout",
                    ailoop_core::models::workflow::OutputType::Stderr => "stderr",
                };
                let content = String::from_utf8_lossy(&chunk.content);

                println!(
                    "[{} | {} | seq:{}]",
                    chunk.state_name, output_type, chunk.chunk_sequence
                );
                print!("{}", content);
                if !content.ends_with('\n') {
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// Handle workflow metrics command
pub async fn handle_workflow_metrics(workflow: Option<String>, json: bool) -> Result<()> {
    let store_path = get_workflow_store_path();
    let persistence = WorkflowPersistence::new(&store_path)
        .context("Failed to initialize workflow persistence")?;

    let metrics = persistence.query_metrics(workflow.as_deref());

    if json {
        // JSON output with calculated rates
        let json_data = serde_json::json!({
            "execution_count": metrics.execution_count,
            "success_count": metrics.success_count,
            "failure_count": metrics.failure_count,
            "success_rate": format!("{:.2}%", metrics.success_rate()),
            "failure_rate": format!("{:.2}%", metrics.failure_rate()),
            "avg_duration_ms": metrics.avg_duration_ms,
        });
        println!("{}", serde_json::to_string_pretty(&json_data)?);
    } else {
        // Human-readable output
        let workflow_name = workflow.as_deref().unwrap_or("All workflows");
        println!("Workflow Metrics: {}", workflow_name);
        println!();
        println!("Total Executions:    {}", metrics.execution_count);
        println!(
            "Successful:          {} ({:.2}%)",
            metrics.success_count,
            metrics.success_rate()
        );
        println!(
            "Failed:              {} ({:.2}%)",
            metrics.failure_count,
            metrics.failure_rate()
        );
        println!("Average Duration:    {} ms", metrics.avg_duration_ms);
        println!();

        if metrics.execution_count > 0 {
            // Calculate success criteria (SC-015: expose metrics)
            println!("Performance Indicators:");
            if metrics.failure_rate() > 10.0 {
                println!("  ⚠️  High failure rate detected");
            } else {
                println!("  ✓  Failure rate within acceptable range");
            }

            if metrics.avg_duration_ms > 300000 {
                // > 5 minutes
                println!("  ⚠️  Average execution time is high");
            } else {
                println!("  ✓  Average execution time is acceptable");
            }
        }
    }

    Ok(())
}
/// Handle workflow validate command
pub async fn handle_workflow_validate(workflow_file: String, json: bool) -> Result<()> {
    use ailoop_core::models::workflow::WorkflowDefinition;
    use ailoop_core::workflow::WorkflowValidator;

    // Read workflow file
    let workflow_content = std::fs::read_to_string(&workflow_file)
        .with_context(|| format!("Failed to read workflow file: {}", workflow_file))?;

    // Parse YAML
    let workflow: WorkflowDefinition =
        serde_yaml::from_str(&workflow_content).context("Failed to parse workflow YAML")?;

    // Validate workflow
    let validation_result =
        WorkflowValidator::validate_workflow(&workflow).context("Failed to validate workflow")?;

    if json {
        // JSON output
        let json_output = serde_json::json!({
            "valid": validation_result.is_valid(),
            "errors": validation_result.errors.iter().map(|e| serde_json::json!({
                "field": e.field,
                "message": e.message,
            })).collect::<Vec<_>>(),
            "warnings": validation_result.warnings,
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        // Human-readable output
        println!("Validating workflow: {}", workflow.name);
        println!("File: {}", workflow_file);
        println!();

        if validation_result.is_valid() {
            println!("✓ Workflow is valid");
            println!();
            println!("Summary:");
            println!("  Name:            {}", workflow.name);
            if let Some(desc) = &workflow.description {
                println!("  Description:     {}", desc);
            }
            println!("  Initial state:   {}", workflow.initial_state);
            println!("  Terminal states: {}", workflow.terminal_states.join(", "));
            println!("  Total states:    {}", workflow.states.len());

            if !validation_result.warnings.is_empty() {
                println!();
                println!("Warnings:");
                for warning in &validation_result.warnings {
                    println!("  ⚠  {}", warning);
                }
            }
        } else {
            println!("✗ Workflow validation failed");
            println!();
            println!("Errors:");
            for error in &validation_result.errors {
                println!("  ✗ {}: {}", error.field, error.message);
            }

            if !validation_result.warnings.is_empty() {
                println!();
                println!("Warnings:");
                for warning in &validation_result.warnings {
                    println!("  ⚠  {}", warning);
                }
            }

            return Err(anyhow::anyhow!("Workflow validation failed"));
        }
    }

    Ok(())
}

/// Handle workflow list-defs command
pub async fn handle_workflow_list_defs(directory: Option<String>, json: bool) -> Result<()> {
    use ailoop_core::models::workflow::WorkflowDefinition;

    // Get workflow definitions directory
    let workflows_dir = if let Some(dir) = directory {
        std::path::PathBuf::from(dir)
    } else {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        home.join(".ailoop").join("workflows")
    };

    // Check if directory exists
    if !workflows_dir.exists() {
        if json {
            println!("{{\"workflows\": []}}");
        } else {
            println!(
                "No workflow definitions directory found at: {}",
                workflows_dir.display()
            );
            println!("Create directory and add workflow YAML files.");
        }
        return Ok(());
    }

    // Find all YAML files
    let mut workflows = Vec::new();
    for entry in std::fs::read_dir(&workflows_dir).context("Failed to read workflows directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && (path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml"))
        {
            // Try to parse workflow
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(workflow) = serde_yaml::from_str::<WorkflowDefinition>(&content) {
                    workflows.push((path, workflow));
                }
            }
        }
    }

    if json {
        // JSON output
        let json_workflows: Vec<_> = workflows
            .iter()
            .map(|(path, workflow)| {
                serde_json::json!({
                    "file": path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                    "path": path.to_string_lossy(),
                    "name": &workflow.name,
                    "description": &workflow.description,
                    "states": workflow.states.len(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "directory": workflows_dir.to_string_lossy(),
                "count": workflows.len(),
                "workflows": json_workflows,
            }))?
        );
    } else {
        // Human-readable output
        println!("Workflow Definitions");
        println!("Directory: {}", workflows_dir.display());
        println!();

        if workflows.is_empty() {
            println!("No workflow definitions found.");
        } else {
            println!("Found {} workflow(s):", workflows.len());
            println!();

            for (path, workflow) in &workflows {
                println!(
                    "  • {} ({})",
                    workflow.name,
                    path.file_name().and_then(|s| s.to_str()).unwrap_or("")
                );
                if let Some(desc) = &workflow.description {
                    println!("    Description: {}", desc);
                }
                println!("    States: {}", workflow.states.len());
                println!();
            }
        }
    }

    Ok(())
}
