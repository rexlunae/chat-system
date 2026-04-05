use chat_system::messengers::{
    DiscordMessenger, GoogleChatMessenger, IMessageMessenger, SlackMessenger, TeamsMessenger,
    TelegramMessenger, WebhookMessenger,
};
use chat_system::Messenger;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

/// Starts a minimal HTTP/1.1 server that reads each request and responds with the given
/// `status_code` and `body`. Returns the base URL (e.g. `http://127.0.0.1:PORT`).
async fn start_mock_http_server(status_code: u16, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let body = body;
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let _ = stream.read(&mut buf).await;
                let status_text = if status_code < 400 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_code, status_text, body.len(), body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    format!("http://127.0.0.1:{}", addr.port())
}

#[derive(Default)]
struct MockDiscordState {
    sent_messages: Mutex<Vec<(String, String)>>,
    typing_channels: Mutex<Vec<String>>,
}

#[derive(Default)]
struct MockSlackState {
    sent_messages: Mutex<Vec<(String, String)>>,
    history_requests: Mutex<Vec<String>>,
}

#[derive(Default)]
struct MockTeamsState {
    sent_messages: Mutex<Vec<(String, String)>>,
    message_list_requests: Mutex<usize>,
}

async fn start_mock_discord_gateway_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut websocket = accept_async(stream).await.unwrap();

        websocket
            .send(WsMessage::Text(
                serde_json::json!({
                    "op": 10,
                    "d": { "heartbeat_interval": 50 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

        if let Some(Ok(WsMessage::Text(payload))) = websocket.next().await {
            let identify: Value = serde_json::from_str(&payload).unwrap();
            assert_eq!(identify["op"].as_i64(), Some(2));
        } else {
            return;
        }

        websocket
            .send(WsMessage::Text(
                serde_json::json!({
                    "op": 0,
                    "t": "MESSAGE_CREATE",
                    "s": 1,
                    "d": {
                        "id": "gateway-message-1",
                        "channel_id": "channel-123",
                        "content": "hello from gateway",
                        "timestamp": "2024-01-01T00:00:00Z",
                        "author": { "username": "gateway-user" },
                        "guild_id": "guild-1"
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;
    });

    format!("ws://127.0.0.1:{}/gateway", addr.port())
}

async fn start_mock_discord_http_server(gateway_url: String) -> (String, Arc<MockDiscordState>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(MockDiscordState::default());
    let state_for_server = state.clone();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let state = state_for_server.clone();
            let gateway_url = gateway_url.clone();

            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let bytes_read = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..bytes_read]);
                let mut lines = request.lines();
                let request_line = lines.next().unwrap_or_default();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default();
                let path = parts.next().unwrap_or_default();
                let body = request
                    .split("\r\n\r\n")
                    .nth(1)
                    .unwrap_or_default();

                let (status_code, status_text, response_body) = match (method, path) {
                    ("GET", "/users/@me") => (200, "OK", r#"{"id":"bot-1","username":"bot"}"#.to_string()),
                    ("GET", "/gateway/bot") => (
                        200,
                        "OK",
                        serde_json::json!({ "url": gateway_url }).to_string(),
                    ),
                    ("POST", path) if path.starts_with("/channels/") && path.ends_with("/messages") => {
                        let channel = path
                            .trim_start_matches("/channels/")
                            .trim_end_matches("/messages")
                            .trim_end_matches('/')
                            .to_string();
                        let payload: Value = serde_json::from_str(body).unwrap_or(Value::Null);
                        let content = payload["content"].as_str().unwrap_or_default().to_string();
                        state.sent_messages.lock().await.push((channel, content));
                        (200, "OK", r#"{"id":"discord-message-42"}"#.to_string())
                    }
                    ("POST", path) if path.starts_with("/channels/") && path.ends_with("/typing") => {
                        let channel = path
                            .trim_start_matches("/channels/")
                            .trim_end_matches("/typing")
                            .trim_end_matches('/')
                            .to_string();
                        state.typing_channels.lock().await.push(channel);
                        (204, "No Content", String::new())
                    }
                    _ => (404, "Not Found", r#"{"error":"not found"}"#.to_string()),
                };

                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_code,
                    status_text,
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    (format!("http://127.0.0.1:{}", addr.port()), state)
}

async fn create_initialized_discord_messenger() -> (DiscordMessenger, Arc<MockDiscordState>) {
    let gateway_url = start_mock_discord_gateway_server().await;
    let (api_base_url, state) = start_mock_discord_http_server(gateway_url).await;
    let mut messenger = DiscordMessenger::new("discord".to_string(), "fake-token".to_string())
        .with_api_base_url(api_base_url);

    messenger.initialize().await.unwrap();
    (messenger, state)
}

async fn wait_for_discord_messages(messenger: &DiscordMessenger) -> Vec<chat_system::Message> {
    for _ in 0..20 {
        let messages = messenger.receive_messages().await.unwrap();
        if !messages.is_empty() {
            return messages;
        }
        sleep(Duration::from_millis(25)).await;
    }

    Vec::new()
}

async fn start_mock_slack_server() -> (String, Arc<MockSlackState>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(MockSlackState::default());
    let state_for_server = state.clone();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let state = state_for_server.clone();

            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let bytes_read = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..bytes_read]);
                let mut lines = request.lines();
                let request_line = lines.next().unwrap_or_default();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default();
                let path = parts.next().unwrap_or_default();
                let normalized_path = path.strip_prefix("/api").unwrap_or(path);
                let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();

                let (status_code, status_text, response_body) = match (method, normalized_path) {
                    ("GET", "/auth.test") => (
                        200,
                        "OK",
                        r#"{"ok":true,"user_id":"U123","team":"Test Workspace"}"#.to_string(),
                    ),
                    ("POST", "/chat.postMessage") => {
                        let payload: Value = serde_json::from_str(body).unwrap_or(Value::Null);
                        let channel = payload["channel"].as_str().unwrap_or_default().to_string();
                        let text = payload["text"].as_str().unwrap_or_default().to_string();
                        state.sent_messages.lock().await.push((channel, text));
                        (200, "OK", r#"{"ok":true,"ts":"1700000001.000100"}"#.to_string())
                    }
                    ("GET", path) if path.starts_with("/conversations.list") => (
                        200,
                        "OK",
                        r#"{"ok":true,"channels":[{"id":"C123"},{"id":"D456"}]}"#.to_string(),
                    ),
                    ("GET", path) if path.starts_with("/conversations.history") => {
                        state.history_requests.lock().await.push(path.to_string());

                        if path.contains("channel=C123") && !path.contains("oldest=") {
                            (
                                200,
                                "OK",
                                r#"{"ok":true,"messages":[{"ts":"1700000002.000200","user":"U456","text":"second channel message"},{"ts":"1700000001.000100","user":"U123","text":"first channel message"}]}"#.to_string(),
                            )
                        } else if path.contains("channel=D456") && !path.contains("oldest=") {
                            (
                                200,
                                "OK",
                                r#"{"ok":true,"messages":[{"ts":"1700000003.000300","user":"U789","text":"direct hello"}]}"#.to_string(),
                            )
                        } else {
                            (200, "OK", r#"{"ok":true,"messages":[]}"#.to_string())
                        }
                    }
                    _ => (404, "Not Found", r#"{"ok":false,"error":"not_found"}"#.to_string()),
                };

                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_code,
                    status_text,
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    (format!("http://127.0.0.1:{}/api", addr.port()), state)
}

async fn create_initialized_slack_messenger() -> (SlackMessenger, Arc<MockSlackState>) {
    let (api_base_url, state) = start_mock_slack_server().await;
    let mut messenger = SlackMessenger::new("slack".to_string(), "fake-token".to_string())
        .with_api_base_url(api_base_url);

    messenger.initialize().await.unwrap();
    (messenger, state)
}

async fn start_mock_teams_graph_server() -> (String, Arc<MockTeamsState>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(MockTeamsState::default());
    let state_for_server = state.clone();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let state = state_for_server.clone();

            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let bytes_read = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..bytes_read]);
                let mut lines = request.lines();
                let request_line = lines.next().unwrap_or_default();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default();
                let path = parts.next().unwrap_or_default();
                let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();

                let (status_code, status_text, response_body) = match (method, path) {
                    ("GET", "/me") => (200, "OK", r#"{"id":"bot-1","displayName":"Teams Bot"}"#.to_string()),
                    ("POST", "/teams/team-1/channels/channel-123/messages") => {
                        let payload: Value = serde_json::from_str(body).unwrap_or(Value::Null);
                        let content = payload["body"]["content"].as_str().unwrap_or_default().to_string();
                        state.sent_messages.lock().await.push(("channel-123".to_string(), content));
                        (200, "OK", r#"{"id":"graph-message-3"}"#.to_string())
                    }
                    ("GET", "/teams/team-1/channels/channel-123/messages") => {
                        let mut requests = state.message_list_requests.lock().await;
                        *requests += 1;

                        if *requests == 1 {
                            (
                                200,
                                "OK",
                                r#"{"value":[{"id":"graph-message-2","createdDateTime":"2024-01-01T00:00:02Z","body":{"content":"second teams message"},"from":{"user":{"displayName":"Bob"}}},{"id":"graph-message-1","createdDateTime":"2024-01-01T00:00:01Z","body":{"content":"first teams message"},"from":{"user":{"displayName":"Alice"}}}]}"#.to_string(),
                            )
                        } else {
                            (
                                200,
                                "OK",
                                r#"{"value":[{"id":"graph-message-2","createdDateTime":"2024-01-01T00:00:02Z","body":{"content":"second teams message"},"from":{"user":{"displayName":"Bob"}}},{"id":"graph-message-1","createdDateTime":"2024-01-01T00:00:01Z","body":{"content":"first teams message"},"from":{"user":{"displayName":"Alice"}}}]}"#.to_string(),
                            )
                        }
                    }
                    _ => (404, "Not Found", r#"{"error":"not found"}"#.to_string()),
                };

                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_code,
                    status_text,
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    (format!("http://127.0.0.1:{}", addr.port()), state)
}

async fn create_initialized_graph_teams_messenger() -> (TeamsMessenger, Arc<MockTeamsState>) {
    let (graph_base_url, state) = start_mock_teams_graph_server().await;
    let mut messenger = TeamsMessenger::new_graph(
        "teams",
        "fake-token",
        "team-1",
        "channel-123",
    )
    .with_graph_base_url(graph_base_url);

    messenger.initialize().await.unwrap();
    (messenger, state)
}

// ─── WebhookMessenger ────────────────────────────────────────────────────────

#[tokio::test]
async fn webhook_name_and_type() {
    let m = WebhookMessenger::new("my-webhook".to_string(), "http://example.com".to_string());
    assert_eq!(m.name(), "my-webhook");
    assert_eq!(m.messenger_type(), "webhook");
}

#[tokio::test]
async fn webhook_not_connected_before_initialize() {
    let m = WebhookMessenger::new("wh".to_string(), "http://example.com".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn webhook_initialize_sets_connected() {
    let mut m = WebhookMessenger::new("wh".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    assert!(m.is_connected());
}

#[tokio::test]
async fn webhook_disconnect_clears_connected() {
    let mut m = WebhookMessenger::new("wh".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

#[tokio::test]
async fn webhook_send_message_success() {
    let url = start_mock_http_server(200, "{}").await;
    let mut m = WebhookMessenger::new("wh".to_string(), url);
    m.initialize().await.unwrap();
    let id = m.send_message("recipient", "hello webhook").await.unwrap();
    assert!(id.starts_with("webhook:"));
}

#[tokio::test]
async fn webhook_send_message_server_error_returns_err() {
    let url = start_mock_http_server(500, "internal error").await;
    let mut m = WebhookMessenger::new("wh".to_string(), url);
    m.initialize().await.unwrap();
    let result = m.send_message("recipient", "test").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn webhook_receive_messages_returns_empty() {
    let mut m = WebhookMessenger::new("wh".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

// ─── TeamsMessenger ───────────────────────────────────────────────────────────

#[tokio::test]
async fn teams_name_and_type() {
    let m = TeamsMessenger::new("my-teams".to_string(), "http://example.com".to_string());
    assert_eq!(m.name(), "my-teams");
    assert_eq!(m.messenger_type(), "msteams");
}

#[tokio::test]
async fn teams_not_connected_before_initialize() {
    let m = TeamsMessenger::new("teams".to_string(), "http://example.com".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn teams_initialize_sets_connected() {
    let mut m = TeamsMessenger::new("teams".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    assert!(m.is_connected());
}

#[tokio::test]
async fn teams_graph_initialize_sets_connected() {
    let (messenger, _) = create_initialized_graph_teams_messenger().await;
    assert!(messenger.is_connected());
}

#[tokio::test]
async fn teams_disconnect_clears_connected() {
    let mut m = TeamsMessenger::new("teams".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

#[tokio::test]
async fn teams_send_message_success() {
    let url = start_mock_http_server(200, "1").await;
    let mut m = TeamsMessenger::new("teams".to_string(), url);
    m.initialize().await.unwrap();
    let id = m.send_message("", "hello teams").await.unwrap();
    assert!(id.starts_with("teams:"));
}

#[tokio::test]
async fn teams_graph_send_message_posts_to_messages_endpoint() {
    let (messenger, state) = create_initialized_graph_teams_messenger().await;

    let id = messenger.send_message("", "hello teams graph").await.unwrap();

    assert_eq!(id, "graph-message-3");
    let sent_messages = state.sent_messages.lock().await;
    assert_eq!(sent_messages.as_slice(), &[("channel-123".to_string(), "hello teams graph".to_string())]);
}

#[tokio::test]
async fn teams_send_message_server_error_returns_err() {
    let url = start_mock_http_server(500, "error").await;
    let mut m = TeamsMessenger::new("teams".to_string(), url);
    m.initialize().await.unwrap();
    let result = m.send_message("", "test").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn teams_receive_returns_empty() {
    let mut m = TeamsMessenger::new("teams".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn teams_graph_receive_messages_polls_without_duplicates() {
    let (messenger, state) = create_initialized_graph_teams_messenger().await;

    let first_poll = messenger.receive_messages().await.unwrap();
    assert_eq!(first_poll.len(), 2);
    assert_eq!(first_poll[0].id, "graph-message-1");
    assert_eq!(first_poll[0].sender, "Alice");
    assert_eq!(first_poll[1].id, "graph-message-2");
    assert_eq!(first_poll[1].sender, "Bob");
    assert_eq!(first_poll[1].channel.as_deref(), Some("channel-123"));

    let second_poll = messenger.receive_messages().await.unwrap();
    assert!(second_poll.is_empty());

    let requests = *state.message_list_requests.lock().await;
    assert!(requests >= 2);
}

// ─── GoogleChatMessenger ──────────────────────────────────────────────────────

#[tokio::test]
async fn google_chat_name_and_type() {
    let m = GoogleChatMessenger::new("my-gchat".to_string(), "http://example.com".to_string());
    assert_eq!(m.name(), "my-gchat");
    assert_eq!(m.messenger_type(), "googlechat");
}

#[tokio::test]
async fn google_chat_not_connected_before_initialize() {
    let m = GoogleChatMessenger::new("gchat".to_string(), "http://example.com".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn google_chat_initialize_sets_connected() {
    let mut m = GoogleChatMessenger::new("gchat".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    assert!(m.is_connected());
}

#[tokio::test]
async fn google_chat_disconnect_clears_connected() {
    let mut m = GoogleChatMessenger::new("gchat".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

#[tokio::test]
async fn google_chat_send_message_success() {
    let url = start_mock_http_server(200, "{}").await;
    let mut m = GoogleChatMessenger::new("gchat".to_string(), url);
    m.initialize().await.unwrap();
    let id = m.send_message("space", "hello google chat").await.unwrap();
    assert!(id.starts_with("googlechat:"));
}

#[tokio::test]
async fn google_chat_send_message_server_error_returns_err() {
    let url = start_mock_http_server(500, "error").await;
    let mut m = GoogleChatMessenger::new("gchat".to_string(), url);
    m.initialize().await.unwrap();
    let result = m.send_message("space", "test").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn google_chat_receive_returns_empty() {
    let mut m = GoogleChatMessenger::new("gchat".to_string(), "http://example.com".to_string());
    m.initialize().await.unwrap();
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

// ─── DiscordMessenger (state management; API URL is hardcoded) ────────────────

#[tokio::test]
async fn discord_name_and_type() {
    let m = DiscordMessenger::new("my-discord".to_string(), "fake-token".to_string());
    assert_eq!(m.name(), "my-discord");
    assert_eq!(m.messenger_type(), "discord");
}

#[tokio::test]
async fn discord_not_connected_before_initialize() {
    let m = DiscordMessenger::new("discord".to_string(), "fake-token".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn discord_initialize_sets_connected_and_receives_gateway_messages() {
    let (messenger, _) = create_initialized_discord_messenger().await;
    assert!(messenger.is_connected());

    let messages = wait_for_discord_messages(&messenger).await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "gateway-message-1");
    assert_eq!(messages[0].sender, "gateway-user");
    assert_eq!(messages[0].content, "hello from gateway");
    assert_eq!(messages[0].channel.as_deref(), Some("channel-123"));
    assert!(!messages[0].is_direct);
}

#[tokio::test]
async fn discord_send_message_posts_to_channel_endpoint() {
    let (messenger, state) = create_initialized_discord_messenger().await;

    let message_id = messenger
        .send_message("channel-123", "hello discord")
        .await
        .unwrap();

    assert_eq!(message_id, "discord-message-42");
    let sent_messages = state.sent_messages.lock().await;
    assert_eq!(sent_messages.as_slice(), &[("channel-123".to_string(), "hello discord".to_string())]);
}

#[tokio::test]
async fn discord_set_typing_posts_typing_indicator() {
    let (messenger, state) = create_initialized_discord_messenger().await;

    messenger.set_typing("channel-123", true).await.unwrap();

    let typing_channels = state.typing_channels.lock().await;
    assert_eq!(typing_channels.as_slice(), &["channel-123".to_string()]);
}

#[tokio::test]
async fn discord_receive_messages_returns_empty_without_gateway() {
    let m = DiscordMessenger::new("discord".to_string(), "fake-token".to_string());
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn discord_disconnect_without_init_is_ok() {
    let mut m = DiscordMessenger::new("discord".to_string(), "fake-token".to_string());
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

#[tokio::test]
async fn discord_disconnect_after_initialize_clears_connected() {
    let (mut messenger, _) = create_initialized_discord_messenger().await;

    messenger.disconnect().await.unwrap();

    assert!(!messenger.is_connected());
}

// ─── TelegramMessenger (state management; API URL is hardcoded) ──────────────

#[tokio::test]
async fn telegram_name_and_type() {
    let m = TelegramMessenger::new("my-telegram".to_string(), "fake-token".to_string());
    assert_eq!(m.name(), "my-telegram");
    assert_eq!(m.messenger_type(), "telegram");
}

#[tokio::test]
async fn telegram_not_connected_before_initialize() {
    let m = TelegramMessenger::new("telegram".to_string(), "fake-token".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn telegram_disconnect_without_init_is_ok() {
    let mut m = TelegramMessenger::new("telegram".to_string(), "fake-token".to_string());
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

// ─── SlackMessenger ───────────────────────────────────────────────────────────

#[tokio::test]
async fn slack_name_and_type() {
    let m = SlackMessenger::new("my-slack".to_string(), "fake-token".to_string());
    assert_eq!(m.name(), "my-slack");
    assert_eq!(m.messenger_type(), "slack");
}

#[tokio::test]
async fn slack_not_connected_before_initialize() {
    let m = SlackMessenger::new("slack".to_string(), "fake-token".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn slack_initialize_sets_connected() {
    let (messenger, _) = create_initialized_slack_messenger().await;
    assert!(messenger.is_connected());
}

#[tokio::test]
async fn slack_send_message_posts_to_chat_api() {
    let (messenger, state) = create_initialized_slack_messenger().await;

    let ts = messenger.send_message("C123", "hello slack").await.unwrap();

    assert_eq!(ts, "1700000001.000100");
    let sent_messages = state.sent_messages.lock().await;
    assert_eq!(sent_messages.as_slice(), &[("C123".to_string(), "hello slack".to_string())]);
}

#[tokio::test]
async fn slack_receive_messages_polls_history_without_duplicates() {
    let (messenger, state) = create_initialized_slack_messenger().await;

    let first_poll = messenger.receive_messages().await.unwrap();
    assert_eq!(first_poll.len(), 3);
    assert_eq!(first_poll[0].id, "1700000001.000100");
    assert_eq!(first_poll[0].sender, "U123");
    assert_eq!(first_poll[0].content, "first channel message");
    assert_eq!(first_poll[0].channel.as_deref(), Some("C123"));
    assert_eq!(first_poll[1].id, "1700000002.000200");
    assert_eq!(first_poll[2].channel.as_deref(), Some("D456"));

    let second_poll = messenger.receive_messages().await.unwrap();
    assert!(second_poll.is_empty());

    let history_requests = state.history_requests.lock().await;
    assert!(history_requests.iter().any(|path| path.contains("channel=C123") && path.contains("oldest=1700000002.000200")));
    assert!(history_requests.iter().any(|path| path.contains("channel=D456") && path.contains("oldest=1700000003.000300")));
}

#[tokio::test]
async fn slack_disconnect_without_init_is_ok() {
    let mut m = SlackMessenger::new("slack".to_string(), "fake-token".to_string());
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

#[tokio::test]
async fn slack_disconnect_after_initialize_clears_connected() {
    let (mut messenger, _) = create_initialized_slack_messenger().await;

    messenger.disconnect().await.unwrap();

    assert!(!messenger.is_connected());
}

// ─── IMessageMessenger (state management; macOS-only for real operations) ────

#[tokio::test]
async fn imessage_name_and_type() {
    let m = IMessageMessenger::new("my-imessage".to_string());
    assert_eq!(m.name(), "my-imessage");
    assert_eq!(m.messenger_type(), "imessage");
}

#[tokio::test]
async fn imessage_not_connected_before_initialize() {
    let m = IMessageMessenger::new("imessage".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn imessage_receive_messages_returns_empty() {
    let m = IMessageMessenger::new("imessage".to_string());
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn imessage_disconnect_without_init_is_ok() {
    let mut m = IMessageMessenger::new("imessage".to_string());
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

/// On non-macOS systems, initialize() must return an error.
#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn imessage_initialize_fails_on_non_macos() {
    let mut m = IMessageMessenger::new("imessage".to_string());
    let result = m.initialize().await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("macOS"), "Error should mention macOS: {}", msg);
}

/// On non-macOS systems, send_message() must return an error.
#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn imessage_send_message_fails_on_non_macos() {
    let m = IMessageMessenger::new("imessage".to_string());
    let result = m.send_message("recipient@example.com", "hello").await;
    assert!(result.is_err());
}
