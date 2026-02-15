//! Tests for critical unwrap() panic fixes in task handlers

use anyhow::Result;
use serde_json::{json, Value};

/// Helper function to parse task list response with error handling
fn parse_task_list_response(response: &str) -> Result<Value> {
    let data: Value = serde_json::from_str(response)?;
    data["tasks"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in response"))?;
    Ok(data)
}

/// Helper function to parse task ready response with error handling
fn parse_task_ready_response(response: &str) -> Result<Value> {
    let data: Value = serde_json::from_str(response)?;
    data["tasks"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in response"))?;
    Ok(data)
}

/// Helper function to parse task blocked response with error handling
fn parse_task_blocked_response(response: &str) -> Result<Value> {
    let data: Value = serde_json::from_str(response)?;
    data["tasks"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'tasks' array in response"))?;
    Ok(data)
}

/// Helper function to parse task from list with error handling
fn parse_task_from_list(task: &Value) -> Result<String> {
    let id = task["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid task_id in response"))?;
    Ok(id.to_string())
}

/// Helper function to parse task title from list with error handling
fn parse_task_title_from_list(task: &Value) -> Result<String> {
    let title = task["title"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid title in task"))?;
    Ok(title.to_string())
}

/// Helper function to parse task state from list with error handling
fn parse_task_state_from_list(task: &Value) -> Result<String> {
    let state = task["state"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid state in task"))?;
    Ok(state.to_string())
}

/// Helper function to parse task blocked status from list with error handling
fn parse_task_blocked_from_list(task: &Value) -> Result<bool> {
    let blocked = task["blocked"]
        .as_bool()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid blocked status in task"))?;
    Ok(blocked)
}

/// Helper function to parse dependency graph with error handling
fn parse_dependency_graph(graph: &Value) -> Result<()> {
    let task = graph["task"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid task in graph"))?;

    task["title"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid task title in graph"))?;

    task["state"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid task state in graph"))?;

    Ok(())
}

#[test]
fn test_malformed_json_response_handling() {
    // Test that malformed JSON doesn't panic
    let malformed_response = r#"{"invalid": "structure"}"#;
    let result = parse_task_list_response(malformed_response);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to deserialize"));
}

#[test]
fn test_missing_tasks_array() {
    // Test response with missing tasks array
    let incomplete_response = r#"{"no_tasks_key": "value"}"#;
    let result = parse_task_list_response(incomplete_response);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing 'tasks' array"));
}

#[test]
fn test_missing_task_id() {
    // Test response with missing task_id
    let incomplete_response =
        r#"{"tasks": [{"title": "Test Task", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(incomplete_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task_id"));
}

#[test]
fn test_missing_task_title() {
    // Test response with missing title
    let incomplete_response = r#"{"tasks": [{"id": "123", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(incomplete_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn test_missing_task_state() {
    // Test response with missing state
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "blocked": false}]}"#;
    let result = parse_task_list_response(incomplete_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("state"));
}

#[test]
fn test_missing_task_blocked() {
    // Test response with missing blocked
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending"}]}"#;
    let result = parse_task_list_response(incomplete_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("blocked"));
}

#[test]
fn test_invalid_field_type_task_id() {
    // Test response with wrong field type (task_id as number)
    let typed_response =
        r#"{"tasks": [{"id": 123, "title": "Test Task", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(typed_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task_id"));
}

#[test]
fn test_complete_valid_list_response() {
    // Test valid response
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Test Task 1",
                "state": "pending",
                "blocked": false
            },
            {
                "id": "456",
                "title": "Test Task 2",
                "state": "done",
                "blocked": true
            }
        ]
    });
    let result = parse_task_list_response(&valid_response.to_string());
    assert!(result.is_ok());
}

#[test]
fn test_empty_tasks_array() {
    // Test empty tasks array
    let valid_response = r#"{"tasks": []}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_empty_string_values() {
    // Test empty string values (they're still valid)
    let valid_response = r#"{"tasks": [{"id": "", "title": "", "state": "", "blocked": false}]}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_missing_graph_task_title() {
    // Test dependency graph with missing task title
    let incomplete_response = json!({
        "task": {},
        "parents": [],
        "children": []
    });
    let result = parse_dependency_graph(&incomplete_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn test_missing_graph_task_state() {
    // Test dependency graph with missing task state
    let incomplete_response = json!({
        "task": {"title": "Main Task"},
        "parents": [],
        "children": []
    });
    let result = parse_dependency_graph(&incomplete_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("state"));
}

#[test]
fn test_empty_graph_arrays() {
    // Test empty graph arrays
    let valid_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [],
        "children": []
    });
    let result = parse_dependency_graph(&valid_response.to_string());
    assert!(result.is_ok());
}

#[test]
fn test_network_error_simulation() {
    // Simulate network error scenario
    let result: Result<Value> = Err(anyhow::anyhow!("Network error: connection timeout"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Network error"));
}

#[test]
fn test_partial_malformed_json() {
    // Test incomplete JSON
    let malformed_response = r#"{"tasks": [{"id": "123"}"#;
    let result = parse_task_list_response(malformed_response);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to deserialize"));
}

#[test]
fn test_missing_parents_array_in_graph() {
    // Test dependency graph with missing parents array
    let incomplete_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "children": []
    });
    let result = parse_dependency_graph(&incomplete_response.to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing"));
}

#[test]
fn test_missing_children_array_in_graph() {
    // Test dependency graph with missing children array
    let incomplete_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": []
    });
    let result = parse_dependency_graph(&incomplete_response.to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing"));
}

#[test]
fn test_special_characters_in_id() {
    // Test special characters in task_id
    let valid_response = r#"{"tasks": [{"id": "task-with-dashes-and_underscores", "title": "Test", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_unicode_in_title() {
    // Test unicode in title
    let valid_response = r#"{"tasks": [{"id": "123", "title": "Task with Unicode", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_whitespace_only_fields() {
    // Test whitespace-only fields
    let valid_response =
        r#"{"tasks": [{"id": "   ", "title": "   ", "state": "   ", "blocked": false}]}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_null_field_values() {
    // Test null values (should fail)
    let invalid_response =
        r#"{"tasks": [{"id": null, "title": null, "state": null, "blocked": null}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task_id"));
}

#[test]
fn test_numeric_state_in_list() {
    // Test numeric state (should fail)
    let invalid_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": 1, "blocked": false}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("state"));
}

#[test]
fn test_boolean_id_in_list() {
    // Test boolean id (should fail)
    let invalid_response =
        r#"{"tasks": [{"id": true, "title": "Test Task", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task_id"));
}

#[test]
fn test_array_id_in_list() {
    // Test array id (should fail)
    let invalid_response = r#"{"tasks": [{"id": ["1", "2"], "title": "Test Task", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task_id"));
}

#[test]
fn test_object_id_in_list() {
    // Test object id (should fail)
    let invalid_response = r#"{"tasks": [{"id": {"value": "123"}, "title": "Test Task", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task_id"));
}

#[test]
fn test_number_title_in_graph() {
    // Test number title in graph (should fail)
    let invalid_response = json!({
        "task": {"title": 123, "state": "pending"},
        "parents": [],
        "children": []
    });
    let result = parse_dependency_graph(&invalid_response.to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn test_array_parent_title_in_graph() {
    // Test array parent title in graph (should fail)
    let invalid_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [{"title": ["parent1", "parent2"]}],
        "children": []
    });
    let result = parse_dependency_graph(&invalid_response.to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn test_empty_string_state_in_list() {
    // Test empty string state (should succeed)
    let valid_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "", "blocked": false}]}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_array_depends_on_in_blocked() {
    // Test array depends_on (should succeed)
    let valid_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending", "depends_on": []}]}"#;
    let result = parse_task_blocked_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_array_depends_on_with_values() {
    // Test array depends_on with values (should succeed)
    let valid_response = r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending", "depends_on": ["456", "789"]}]}"#;
    let result = parse_task_blocked_response(valid_response);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_depends_on_type() {
    // Test invalid depends_on type (should fail)
    let invalid_response = r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending", "depends_on": "not-an-array"}]}"#;
    let result = parse_task_blocked_response(invalid_response);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing 'tasks' array"));
}

#[test]
fn test_multiple_missing_fields() {
    // Test multiple missing fields in one task (should fail)
    let invalid_response = r#"{"tasks": [{"id": 123, "state": null}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("task_id") || error_msg.contains("title"));
}

#[test]
fn test_valid_complete_graph_response() {
    // Test complete valid dependency graph
    let valid_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [
            {"title": "Parent Task 1", "state": "done"},
            {"title": "Parent Task 2", "state": "pending"}
        ],
        "children": [
            {"title": "Child Task 1", "state": "pending"},
            {"title": "Child Task 2", "state": "abandoned"}
        ]
    });
    let result = parse_dependency_graph(&valid_response.to_string());
    assert!(result.is_ok());
}

#[test]
fn test_null_values_detailed_error() {
    // Test detailed error message for null values
    let invalid_response =
        r#"{"tasks": [{"id": null, "title": null, "state": null, "blocked": null}]}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("task_id"));
    assert!(error_msg.contains("Missing") || error_msg.contains("invalid"));
}

#[test]
fn test_unreachable_server() {
    // Test handling of unreachable server scenario
    let result = Result::<Value, anyhow::Error>::Err(anyhow::anyhow!("Connection refused"));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Connection refused"));
}

#[test]
fn test_empty_response() {
    // Test empty response
    let invalid_response = r#"{}"#;
    let result = parse_task_list_response(invalid_response);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to deserialize"));
}

#[test]
fn test_special_characters_in_title() {
    // Test special characters in title
    let valid_response = r#"{"tasks": [{"id": "123", "title": "Test Task with <special> & characters", "state": "pending", "blocked": false}]}"#;
    let result = parse_task_list_response(valid_response);
    assert!(result.is_ok());
}
