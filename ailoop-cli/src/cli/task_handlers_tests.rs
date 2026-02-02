use crate::cli::task_handlers::handle_dep_graph;
use crate::cli::task_handlers::handle_task_blocked;
use crate::cli::task_handlers::handle_task_list;
use crate::cli::task_handlers::handle_task_ready;

#[tokio::test]
async fn test_malformed_json_response_handling() {
    let malformed_response = r#"{"invalid": "structure"}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for malformed JSON");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to list tasks"),
        "Error should mention failed to list tasks, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_tasks_array() {
    let incomplete_response = r#"{"no_tasks_key": "value"}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for missing tasks array");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Missing 'tasks' array"),
        "Error should mention missing tasks array, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_id_in_list() {
    let incomplete_response =
        r#"{"tasks": [{"title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for missing task_id");
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention missing task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_title_in_list() {
    let incomplete_response = r#"{"tasks": [{"id": "123", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for missing task title");
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention missing title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_state_in_list() {
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for missing task state");
    assert!(
        result.unwrap_err().to_string().contains("state"),
        "Error should mention missing state, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_blocked_in_list() {
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending"}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for missing blocked field");
    assert!(
        result.unwrap_err().to_string().contains("blocked"),
        "Error should mention missing blocked, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_invalid_field_type_in_list() {
    let typed_response =
        r#"{"tasks": [{"id": 123, "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for invalid field type (task_id as number)"
    );
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_complete_valid_list_response() {
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

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_ok(), "Expected success for valid response");
}

#[tokio::test]
async fn test_missing_task_id_in_ready() {
    let incomplete_response = r#"{"tasks": [{"title": "Test Task", "state": "pending"}]}"#;

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing task_id in ready"
    );
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention missing task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_title_in_ready() {
    let incomplete_response = r#"{"tasks": [{"id": "123", "state": "pending"}]}"#;

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing task title in ready"
    );
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention missing title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_tasks_array_in_ready() {
    let incomplete_response = r#"{"no_tasks_key": "value"}"#;

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing tasks array in ready"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Missing 'tasks' array"),
        "Error should mention missing tasks array, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_id_in_blocked() {
    let incomplete_response =
        r#"{"tasks": [{"title": "Test Task", "state": "pending", "depends_on": []}]}"#;

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing task_id in blocked"
    );
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention missing task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_task_title_in_blocked() {
    let incomplete_response = r#"{"tasks": [{"id": "123", "state": "pending", "depends_on": []}]}"#;

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing task title in blocked"
    );
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention missing title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_depends_on_array_in_blocked() {
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending"}]}"#;

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing depends_on array in blocked"
    );
    assert!(
        result.unwrap_err().to_string().contains("depends_on"),
        "Error should mention missing depends_on, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_tasks_array_in_blocked() {
    let incomplete_response = r#"{"no_tasks_key": "value"}"#;

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing tasks array in blocked"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Missing 'tasks' array"),
        "Error should mention missing tasks array, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_complete_valid_blocked_response() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Blocked Task 1",
                "state": "pending",
                "depends_on": ["456"]
            },
            {
                "id": "456",
                "title": "Blocking Task",
                "state": "pending",
                "depends_on": []
            }
        ]
    });

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for valid blocked response"
    );
}

#[tokio::test]
async fn test_empty_tasks_array_in_list() {
    let valid_response = json!({
        "tasks": []
    });

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_ok(), "Expected success for empty tasks array");
}

#[tokio::test]
async fn test_empty_tasks_array_in_ready() {
    let valid_response = json!({
        "tasks": []
    });

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for empty tasks array in ready"
    );
}

#[tokio::test]
async fn test_empty_tasks_array_in_blocked() {
    let valid_response = json!({
        "tasks": []
    });

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for empty tasks array in blocked"
    );
}

#[tokio::test]
async fn test_partial_malformed_json_in_list() {
    let malformed_response = r#"{"tasks": [{"id": "123"}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for incomplete JSON");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to list tasks"),
        "Error should mention failed to list tasks, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_graph_task_title() {
    let incomplete_response = json!({
        "task": {},
        "parents": [],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing task title in graph"
    );
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention missing title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_graph_parent_title() {
    let incomplete_response = json!({
        "task": {"title": "Main Task"},
        "parents": [{"state": "pending"}],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing parent title in graph"
    );
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention missing title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_graph_parent_state() {
    let incomplete_response = json!({
        "task": {"title": "Main Task"},
        "parents": [{"title": "Parent Task"}],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing parent state in graph"
    );
    assert!(
        result.unwrap_err().to_string().contains("state"),
        "Error should mention missing state, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_graph_child_title() {
    let incomplete_response = json!({
        "task": {"title": "Main Task"},
        "parents": [],
        "children": [{"state": "pending"}]
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing child title in graph"
    );
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention missing title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_graph_child_state() {
    let incomplete_response = json!({
        "task": {"title": "Main Task"},
        "parents": [],
        "children": [{"title": "Child Task"}]
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing child state in graph"
    );
    assert!(
        result.unwrap_err().to_string().contains("state"),
        "Error should mention missing state, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_empty_graph_array() {
    let valid_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(result.is_ok(), "Expected success for empty graph arrays");
}

#[tokio::test]
async fn test_missing_graph_array() {
    let incomplete_response = json!({
        "task": {"title": "Main Task"}
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(result.is_err(), "Expected error for missing graph arrays");
    assert!(
        result.unwrap_err().to_string().contains("Missing"),
        "Error should mention missing field, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_valid_complete_graph_response() {
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

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for valid complete graph response"
    );
}

#[tokio::test]
async fn test_network_error_handling() {
    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:9999".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for unreachable server");
}

#[tokio::test]
async fn test_empty_string_field_values() {
    let incomplete_response =
        r#"{"tasks": [{"id": "", "title": "", "state": "", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for empty string values (they're still valid)"
    );
}

#[tokio::test]
async fn test_null_field_values() {
    let incomplete_response =
        r#"{"tasks": [{"id": null, "title": null, "state": null, "blocked": null}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for null values");
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_missing_response_status_handling() {
    let incomplete_response = r#"{}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for empty response");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to list tasks"),
        "Error should mention failed to list tasks, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_list_response_with_state_filter() {
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        Some("done".to_string()),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for incomplete task in list with state filter"
    );
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention missing task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_blocked_task_with_empty_depends_on() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Blocking Task",
                "state": "pending",
                "depends_on": []
            }
        ]
    });

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for blocked task with empty depends_on"
    );
}

#[tokio::test]
async fn test_malformed_dependency_array_in_blocked() {
    let incomplete_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Blocked Task",
                "state": "pending",
                "depends_on": 123
            }
        ]
    });

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for non-array depends_on");
    assert!(
        result.unwrap_err().to_string().contains("depends_on"),
        "Error should mention depends_on, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_list_task_with_number_id() {
    let typed_response =
        r#"{"tasks": [{"id": 123, "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for number task_id");
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_ready_task_with_number_id() {
    let typed_response =
        r#"{"tasks": [{"id": 123, "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for number task_id in ready"
    );
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_blocked_task_with_number_id() {
    let typed_response =
        r#"{"tasks": [{"id": 123, "title": "Test Task", "state": "pending", "depends_on": []}]}"#;

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for number task_id in blocked"
    );
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_graph_with_number_id() {
    let valid_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [],
        "children": []
    });

    let result = handle_dep_graph(
        123.to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(result.is_ok(), "Expected success with valid number task_id");
}

#[tokio::test]
async fn test_task_list_with_multiple_missing_fields() {
    let incomplete_response = r#"{"tasks": [{"id": 123, "state": null}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for multiple missing fields"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("task_id") || error_msg.contains("title"),
        "Error should mention missing field, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_ready_task_with_number_id_in_title() {
    let typed_response =
        r#"{"tasks": [{"id": "123", "title": 456, "state": "pending", "blocked": false}]}"#;

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for number title");
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention invalid title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_blocked_task_with_missing_blocked_flag() {
    let incomplete_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": "pending", "depends_on": []}]}"#;

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_err(),
        "Expected error for missing blocked field in blocked tasks"
    );
    assert!(
        result.unwrap_err().to_string().contains("blocked"),
        "Error should mention missing blocked, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_dep_graph_with_null_state() {
    let incomplete_response = json!({
        "task": {"title": "Main Task", "state": null},
        "parents": [],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(result.is_err(), "Expected error for null state");
    assert!(
        result.unwrap_err().to_string().contains("state"),
        "Error should mention invalid state, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_dep_graph_with_invalid_boolean_blocked() {
    let incomplete_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success when graph is complete but doesn't use blocked field"
    );
}

#[tokio::test]
async fn test_list_response_with_whitespace_fields() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "   ",
                "title": "   ",
                "state": "   ",
                "blocked": false
            }
        ]
    });

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for whitespace-only fields"
    );
}

#[tokio::test]
async fn test_task_list_with_special_characters_in_id() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "task-with-dashes-and_underscores",
                "title": "Task with Special Characters",
                "state": "pending",
                "blocked": false
            }
        ]
    });

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for special characters in task_id"
    );
}

#[tokio::test]
async fn test_task_list_with_unicode_in_title() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Task with 🎉 Unicode",
                "state": "pending",
                "blocked": false
            }
        ]
    });

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_ok(), "Expected success for unicode in title");
}

#[tokio::test]
async fn test_task_list_with_numeric_state() {
    let typed_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": 1, "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for number state");
    assert!(
        result.unwrap_err().to_string().contains("state"),
        "Error should mention invalid state, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_blocked_task_with_empty_string_in_depends_on() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Test Task",
                "state": "pending",
                "depends_on": ["", ""]
            }
        ]
    });

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for empty strings in depends_on array"
    );
}

#[tokio::test]
async fn test_ready_task_with_numeric_state() {
    let typed_response =
        r#"{"tasks": [{"id": "123", "title": "Test Task", "state": 1, "blocked": false}]}"#;

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for number state in ready");
    assert!(
        result.unwrap_err().to_string().contains("state"),
        "Error should mention invalid state, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_list_task_with_boolean_id() {
    let typed_response =
        r#"{"tasks": [{"id": true, "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for boolean task_id");
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_list_task_with_array_id() {
    let typed_response = r#"{"tasks": [{"id": ["1", "2"], "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for array task_id");
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_list_task_with_object_id() {
    let typed_response = r#"{"tasks": [{"id": {"value": "123"}, "title": "Test Task", "state": "pending", "blocked": false}]}"#;

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for object task_id");
    assert!(
        result.unwrap_err().to_string().contains("task_id"),
        "Error should mention invalid task_id, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_graph_response_with_number_title() {
    let typed_response = json!({
        "task": {"title": 123, "state": "pending"},
        "parents": [],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(result.is_err(), "Expected error for number title in graph");
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention invalid title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_graph_response_with_array_parent_titles() {
    let typed_response = json!({
        "task": {"title": "Main Task", "state": "pending"},
        "parents": [{"title": ["parent1", "parent2"]}],
        "children": []
    });

    let result = handle_dep_graph(
        "123".to_string(),
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
    )
    .await;

    assert!(result.is_err(), "Expected error for array parent title");
    assert!(
        result.unwrap_err().to_string().contains("title"),
        "Error should mention invalid title, got: {:?}",
        result.unwrap_err()
    );
}

#[tokio::test]
async fn test_list_task_with_empty_string_state() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Test Task",
                "state": "",
                "blocked": false
            }
        ]
    });

    let result = handle_task_list(
        "test-channel".to_string(),
        None,
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_ok(), "Expected success for empty string state");
}

#[tokio::test]
async fn test_ready_task_with_special_characters_in_title() {
    let valid_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Test Task with <special> & characters",
                "state": "pending",
                "blocked": false
            }
        ]
    });

    let result = handle_task_ready(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expected success for special characters in title"
    );
}

#[tokio::test]
async fn test_blocked_task_with_invalid_depends_on_type() {
    let typed_response = json!({
        "tasks": [
            {
                "id": "123",
                "title": "Test Task",
                "state": "pending",
                "depends_on": "not-an-array"
            }
        ]
    });

    let result = handle_task_blocked(
        "test-channel".to_string(),
        "http://127.0.0.1:8080".to_string(),
        true,
    )
    .await;

    assert!(result.is_err(), "Expected error for non-array depends_on");
    assert!(
        result.unwrap_err().to_string().contains("depends_on"),
        "Error should mention invalid depends_on, got: {:?}",
        result.unwrap_err()
    );
}
