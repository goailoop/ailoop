use ailoop_core::models::{Message, MessageContent, SenderType};
use ailoop_core::transport::file::FileTransport;
use ailoop_core::transport::Transport;
use tempfile::NamedTempFile;

#[tokio::test]
async fn file_transport_writes_messages() {
    let temp_file = NamedTempFile::new().expect("failed to create temp file");
    let path = temp_file.path().to_path_buf();
    let mut transport = FileTransport::new(path.clone(), "test-channel".to_string())
        .expect("failed to create FileTransport");

    let message = Message::new(
        "test-channel".to_string(),
        SenderType::Agent,
        MessageContent::Question {
            text: "What?".to_string(),
            timeout_seconds: 5,
            choices: None,
        },
    );

    transport.send(message.clone()).await.expect("send failed");
    transport.flush().await.expect("flush failed");
    transport.close().await.expect("close failed");

    let contents = std::fs::read_to_string(&path).expect("failed to read log file");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 1);

    let stored: Message = serde_json::from_str(lines[0]).expect("failed to parse message");
    assert_eq!(stored.channel, "test-channel");
    assert_eq!(stored.sender_type, SenderType::Agent);
    if let MessageContent::Question { text, .. } = stored.content {
        assert_eq!(text, "What?");
    } else {
        panic!("unexpected message content");
    }
}
