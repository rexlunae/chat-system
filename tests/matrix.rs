#![cfg(feature = "matrix")]

use chat_system::messengers::MatrixMessenger;
use chat_system::Messenger;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const MOCK_MATRIX_USER_ID: &str = "@bot:mock.invalid";
const MOCK_MATRIX_SENDER_ID: &str = "@alice:mock.invalid";
const MOCK_MATRIX_ROOM_ALIAS: &str = "#room:mock.invalid";
const MOCK_MATRIX_ROOM_ID: &str = "!room:mock.invalid";

async fn start_mock_matrix_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                let request_line = request.lines().next().unwrap_or_default();

                let (status, body) = if request_line.starts_with("POST /_matrix/client/v3/login ") {
                    (
                        200,
                        r#"{"access_token":"test-token","user_id":"@bot:mock.invalid"}"#,
                    )
                } else if request_line.starts_with("GET /_matrix/client/v3/sync?")
                    && request_line.contains("since=s1")
                {
                    (
                        200,
                        r#"{"next_batch":"s2","rooms":{"join":{"!room:mock.invalid":{"timeline":{"events":[{"type":"m.room.message","event_id":"$event-1","sender":"@alice:mock.invalid","origin_server_ts":1712000000000,"content":{"body":"hello from matrix"}}]}}}}}"#,
                    )
                } else if request_line.starts_with("GET /_matrix/client/v3/sync?") {
                    (200, r#"{"next_batch":"s1","rooms":{"join":{}}}"#)
                } else if request_line.starts_with("POST /_matrix/client/v3/join/") {
                    (200, r#"{"room_id":"!room:mock.invalid"}"#)
                } else if request_line.contains("/send/m.room.message/") {
                    (200, r#"{"event_id":"$sent-event"}"#)
                } else if request_line.contains("/typing/") {
                    (200, "{}")
                } else if request_line.starts_with("POST /_matrix/client/v3/logout ") {
                    (200, "{}")
                } else {
                    (404, r#"{"errcode":"M_NOT_FOUND","error":"not found"}"#)
                };

                let status_text = if status < 400 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    status_text,
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    format!("http://127.0.0.1:{}", addr.port())
}

#[tokio::test]
async fn matrix_name_and_type() {
    let m = MatrixMessenger::new("matrix", "http://localhost:8008", MOCK_MATRIX_USER_ID, "secret");
    assert_eq!(m.name(), "matrix");
    assert_eq!(m.messenger_type(), "matrix");
}

#[tokio::test]
async fn matrix_initialize_sets_connected() {
    let homeserver = start_mock_matrix_server().await;
    let mut m = MatrixMessenger::new("matrix", homeserver, MOCK_MATRIX_USER_ID, "secret");
    m.initialize().await.unwrap();
    assert!(m.is_connected());
}

#[tokio::test]
async fn matrix_send_message_joins_alias_and_returns_event_id() {
    let homeserver = start_mock_matrix_server().await;
    let mut m = MatrixMessenger::new("matrix", homeserver, MOCK_MATRIX_USER_ID, "secret");
    m.initialize().await.unwrap();
    let id = m.send_message(MOCK_MATRIX_ROOM_ALIAS, "hello matrix").await.unwrap();
    assert_eq!(id, "$sent-event");
}

#[tokio::test]
async fn matrix_receive_messages_returns_synced_messages() {
    let homeserver = start_mock_matrix_server().await;
    let mut m = MatrixMessenger::new("matrix", homeserver, MOCK_MATRIX_USER_ID, "secret");
    m.initialize().await.unwrap();

    let messages = m.receive_messages().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "$event-1");
    assert_eq!(messages[0].sender, MOCK_MATRIX_SENDER_ID);
    assert_eq!(messages[0].content, "hello from matrix");
    assert_eq!(messages[0].channel.as_deref(), Some(MOCK_MATRIX_ROOM_ID));
}

#[tokio::test]
async fn matrix_disconnect_clears_connected() {
    let homeserver = start_mock_matrix_server().await;
    let mut m = MatrixMessenger::new("matrix", homeserver, MOCK_MATRIX_USER_ID, "secret");
    m.initialize().await.unwrap();
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}
