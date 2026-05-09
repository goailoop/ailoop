use ailoop_core::models::{DecisionOption, Message, MessageContent, SenderType};
use ailoop_core::transport::websocket::WebSocketTransport;
use ailoop_core::transport::Transport;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

#[tokio::test]
async fn websocket_transport_connects_and_send() {
    let addr = "127.0.0.1:28180";
    let (ready_tx, ready_rx) = oneshot::channel();
    let (message_tx, message_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        let listener = TcpListener::bind(addr)
            .await
            .expect("failed to bind websocket server");
        let _ = ready_tx.send(());
        let (stream, _) = listener
            .accept()
            .await
            .expect("failed to accept connection");
        let ws_stream = accept_async(stream)
            .await
            .expect("failed to upgrade to websocket");
        let (mut sender, mut receiver) = ws_stream.split();
        if let Some(Ok(WsMessage::Text(text))) = receiver.next().await {
            let _ = message_tx.send(text);
        }
        let _ = sender.close().await;
    });

    ready_rx.await.expect("server ready signal failed");

    let mut transport =
        WebSocketTransport::new(format!("ws://{}", addr), "test-channel".to_string(), None)
            .expect("failed to create websocket transport");

    let message = Message::new(
        "test-channel".to_string(),
        SenderType::Agent,
        MessageContent::Decision {
            decision_id: "ping".to_string(),
            summary: "Ping".to_string(),
            context_markdown: None,
            options: vec![
                DecisionOption {
                    id: "a".to_string(),
                    label: "A".to_string(),
                    detail_markdown: None,
                },
                DecisionOption {
                    id: "b".to_string(),
                    label: "B".to_string(),
                    detail_markdown: None,
                },
            ],
            recommendation: None,
            timeout_seconds: 5,
        },
    );

    transport.send(message.clone()).await.expect("send failed");
    transport.flush().await.expect("flush failed");
    transport.close().await.expect("close failed");

    let received_text = message_rx
        .await
        .expect("failed to receive message from server");
    let stored: Message = serde_json::from_str(&received_text).expect("failed to parse message");
    assert_eq!(stored.channel, "test-channel");
    assert!(matches!(stored.content, MessageContent::Decision { .. }));

    server_handle.await.expect("server panic");
}
