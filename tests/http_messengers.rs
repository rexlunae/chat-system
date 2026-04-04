use chat_system::messengers::{
    DiscordMessenger, GoogleChatMessenger, IMessageMessenger, SlackMessenger, TeamsMessenger,
    TelegramMessenger, WebhookMessenger,
};
use chat_system::Messenger;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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
    let mut m =
        GoogleChatMessenger::new("gchat".to_string(), "http://example.com".to_string());
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

// ─── SlackMessenger (state management; API URL is hardcoded) ─────────────────

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
async fn slack_receive_messages_returns_empty() {
    let m = SlackMessenger::new("slack".to_string(), "fake-token".to_string());
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn slack_disconnect_without_init_is_ok() {
    let mut m = SlackMessenger::new("slack".to_string(), "fake-token".to_string());
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
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
