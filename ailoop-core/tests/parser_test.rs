use ailoop_core::parser::{create_parser, EventType, InputFormat};

#[tokio::test]
async fn test_cursor_parser_stream_json() {
    let mut parser = create_parser(Some("cursor".to_string()), InputFormat::StreamJson)
        .expect("Failed to create cursor parser");

    let line = r#"{"type":"assistant","session_id":"sess-123","message":"Hello world"}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::Assistant);
    assert_eq!(event._agent_type, "cursor");
    assert_eq!(
        event.metadata.get("session_id"),
        Some(&"sess-123".to_string())
    );
    assert!(event.content.get("message").is_some());
}

#[tokio::test]
async fn test_cursor_parser_text() {
    let mut parser = create_parser(Some("cursor".to_string()), InputFormat::Text)
        .expect("Failed to create cursor parser");

    let line = "This is plain text output";
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::Assistant);
    assert_eq!(event._agent_type, "cursor");
    assert_eq!(
        event.content.get("text").and_then(|v| v.as_str()),
        Some("This is plain text output")
    );
    assert_eq!(
        event.content.get("format").and_then(|v| v.as_str()),
        Some("text")
    );
}

#[tokio::test]
async fn test_cursor_parser_empty_line() {
    let mut parser = create_parser(Some("cursor".to_string()), InputFormat::Text)
        .expect("Failed to create cursor parser");

    let event = parser
        .parse_line("")
        .await
        .expect("Failed to parse empty line");
    assert!(event.is_none());

    let event = parser
        .parse_line("   ")
        .await
        .expect("Failed to parse whitespace line");
    assert!(event.is_none());
}

#[tokio::test]
async fn test_jsonl_parser_with_agent_type() {
    let mut parser = create_parser(Some("jsonl".to_string()), InputFormat::StreamJson)
        .expect("Failed to create jsonl parser");

    let line = r#"{"agent_type":"claude","type":"user","session_id":"sess-456","client_id":"client-789","content":"Hello"}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::User);
    assert_eq!(event._agent_type, "claude");
    assert_eq!(
        event.metadata.get("session_id"),
        Some(&"sess-456".to_string())
    );
    assert_eq!(
        event.metadata.get("client_id"),
        Some(&"client-789".to_string())
    );
}

#[tokio::test]
async fn test_jsonl_parser_auto_detect() {
    let mut parser =
        create_parser(None, InputFormat::StreamJson).expect("Failed to create jsonl parser");

    let line = r#"{"agent_type":"gpt","type":"assistant","message":"Response"}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::Assistant);
    assert_eq!(event._agent_type, "gpt");
}

#[tokio::test]
async fn test_jsonl_parser_empty_line() {
    let mut parser = create_parser(Some("jsonl".to_string()), InputFormat::StreamJson)
        .expect("Failed to create jsonl parser");

    let event = parser
        .parse_line("")
        .await
        .expect("Failed to parse empty line");
    assert!(event.is_none());
}

#[tokio::test]
async fn test_opencode_parser_text_event() {
    let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)
        .expect("Failed to create opencode parser");

    let line = r#"{"type":"text","timestamp":1700000000000,"sessionID":"sess-1","part":{"type":"text","text":"Hello from opencode"}}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::Assistant);
    assert_eq!(event._agent_type, "opencode");
    assert_eq!(
        event.content.get("message").and_then(|v| v.as_str()),
        Some("Hello from opencode")
    );
}

#[tokio::test]
async fn test_opencode_parser_tool_use() {
    let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)
        .expect("Failed to create opencode parser");

    let line = r#"{"type":"tool_use","timestamp":1700000002000,"sessionID":"sess-1","part":{"tool":"shell","state":{"status":"completed","input":{"cmd":"ls"},"output":"done"}}}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::ToolCall);
    assert_eq!(event._agent_type, "opencode");
    assert_eq!(
        event.content.get("tool").and_then(|v| v.as_str()),
        Some("shell")
    );
    assert_eq!(
        event.content.get("status").and_then(|v| v.as_str()),
        Some("completed")
    );
}

#[tokio::test]
async fn test_opencode_parser_step_start() {
    let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)
        .expect("Failed to create opencode parser");

    let line = r#"{"type":"step_start","timestamp":1700000000000,"sessionID":"sess-1","part":{"type":"step-start","snapshot":{}}}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::System);
    assert_eq!(event._agent_type, "opencode");
    assert_eq!(
        event.metadata.get("session_id"),
        Some(&"sess-1".to_string())
    );
}

#[tokio::test]
async fn test_opencode_parser_step_finish_stop() {
    let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)
        .expect("Failed to create opencode parser");

    let line = r#"{"type":"step_finish","timestamp":1700000003000,"sessionID":"sess-1","part":{"type":"step-finish","reason":"stop","cost":12.3,"tokens":123}}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::Result);
    assert_eq!(event._agent_type, "opencode");
    assert_eq!(
        event.content.get("result").and_then(|v| v.as_str()),
        Some("complete")
    );
    assert_eq!(
        event.content.get("duration").and_then(|v| v.as_f64()),
        Some(12.3)
    );
}

#[tokio::test]
async fn test_opencode_parser_error() {
    let mut parser = create_parser(Some("opencode".to_string()), InputFormat::StreamJson)
        .expect("Failed to create opencode parser");

    let line = r#"{"type":"error","timestamp":1700000004000,"sessionID":"sess-1","error":{"name":"ToolError","data":{"message":"Something went wrong"}}}"#;
    let event = parser.parse_line(line).await.expect("Failed to parse line");

    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, EventType::Error);
    assert_eq!(event._agent_type, "opencode");
    assert_eq!(
        event.content.get("message").and_then(|v| v.as_str()),
        Some("Something went wrong")
    );
}

#[tokio::test]
async fn test_opencode_parser_text_format_rejected() {
    let result = create_parser(Some("opencode".to_string()), InputFormat::Text);
    assert!(result.is_err());
    match result {
        Err(e) => assert!(e.to_string().contains("does not support text format")),
        _ => panic!("Expected error for opencode with Text format"),
    }
}

#[tokio::test]
async fn test_unknown_agent_type() {
    let result = create_parser(Some("unknown_agent".to_string()), InputFormat::StreamJson);
    assert!(result.is_err());
    match result {
        Err(e) => assert!(e.to_string().contains("Unknown agent type")),
        _ => panic!("Expected error for unknown agent type"),
    }
}

#[tokio::test]
async fn test_cursor_parser_event_types() {
    let mut parser = create_parser(Some("cursor".to_string()), InputFormat::StreamJson)
        .expect("Failed to create cursor parser");

    let test_cases = vec![
        (r#"{"type":"system"}"#, EventType::System),
        (r#"{"type":"user"}"#, EventType::User),
        (r#"{"type":"assistant"}"#, EventType::Assistant),
        (r#"{"type":"tool_call"}"#, EventType::ToolCall),
        (r#"{"type":"result"}"#, EventType::Result),
        (r#"{"type":"error"}"#, EventType::Error),
        (
            r#"{"type":"custom_event"}"#,
            EventType::Custom("custom_event".to_string()),
        ),
    ];

    for (line, expected_type) in test_cases {
        let event = parser.parse_line(line).await.expect("Failed to parse line");
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_type, expected_type);
    }
}

#[tokio::test]
async fn test_jsonl_parser_event_types() {
    let mut parser = create_parser(Some("jsonl".to_string()), InputFormat::StreamJson)
        .expect("Failed to create jsonl parser");

    let test_cases = vec![
        (
            r#"{"agent_type":"test","type":"system"}"#,
            EventType::System,
        ),
        (r#"{"agent_type":"test","type":"user"}"#, EventType::User),
        (
            r#"{"agent_type":"test","type":"assistant"}"#,
            EventType::Assistant,
        ),
        (
            r#"{"agent_type":"test","type":"tool_call"}"#,
            EventType::ToolCall,
        ),
        (
            r#"{"agent_type":"test","type":"result"}"#,
            EventType::Result,
        ),
        (r#"{"agent_type":"test","type":"error"}"#, EventType::Error),
        (
            r#"{"agent_type":"test","type":"my_custom"}"#,
            EventType::Custom("my_custom".to_string()),
        ),
    ];

    for (line, expected_type) in test_cases {
        let event = parser.parse_line(line).await.expect("Failed to parse line");
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_type, expected_type);
    }
}
