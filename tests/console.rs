use chat_system::messengers::ConsoleMessenger;
use chat_system::{Message, Messenger, SendOptions};

fn make_message(id: &str, sender: &str, content: &str) -> Message {
    Message {
        id: id.to_string(),
        sender: sender.to_string(),
        content: content.to_string(),
        timestamp: 1000,
        channel: Some("#general".to_string()),
        reply_to: None,
        media: None,
        is_direct: false,
        reactions: None,
    }
}

#[tokio::test]
async fn console_new_is_not_connected() {
    let m = ConsoleMessenger::new("test".to_string());
    assert!(!m.is_connected());
}

#[tokio::test]
async fn console_name_and_type() {
    let m = ConsoleMessenger::new("my-console".to_string());
    assert_eq!(m.name(), "my-console");
    assert_eq!(m.messenger_type(), "console");
}

#[tokio::test]
async fn console_initialize_sets_connected() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    assert!(m.is_connected());
}

#[tokio::test]
async fn console_disconnect_clears_connected() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    assert!(m.is_connected());
    m.disconnect().await.unwrap();
    assert!(!m.is_connected());
}

#[tokio::test]
async fn console_reinitialize_after_disconnect() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    m.disconnect().await.unwrap();
    m.initialize().await.unwrap();
    assert!(m.is_connected());
}

#[tokio::test]
async fn console_send_message_returns_console_prefixed_id() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    let id = m.send_message("#general", "hello").await.unwrap();
    assert!(id.starts_with("console:"));
}

#[tokio::test]
async fn console_send_message_different_recipients() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    m.send_message("#general", "hello general").await.unwrap();
    m.send_message("alice", "hello alice").await.unwrap();
    m.send_message("#rust", "hello rust").await.unwrap();
}

#[tokio::test]
async fn console_receive_empty_initially() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    let msgs = m.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn console_enqueue_and_receive_single_message() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    m.enqueue(make_message("1", "alice", "Hello"));
    let msgs = m.receive_messages().await.unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].id, "1");
    assert_eq!(msgs[0].sender, "alice");
    assert_eq!(msgs[0].content, "Hello");
}

#[tokio::test]
async fn console_enqueue_multiple_messages() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    for i in 0..5 {
        m.enqueue(make_message(
            &i.to_string(),
            "sender",
            &format!("message {}", i),
        ));
    }
    let msgs = m.receive_messages().await.unwrap();
    assert_eq!(msgs.len(), 5);
}

#[tokio::test]
async fn console_receive_is_idempotent() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    m.enqueue(make_message("1", "alice", "Hello"));
    // ConsoleMessenger clones the queue on each receive
    let msgs1 = m.receive_messages().await.unwrap();
    let msgs2 = m.receive_messages().await.unwrap();
    assert_eq!(msgs1.len(), msgs2.len());
    assert_eq!(msgs1[0].id, msgs2[0].id);
}

#[tokio::test]
async fn console_send_with_options_delegates_to_send_message() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    let opts = SendOptions {
        recipient: "#channel",
        content: "test message",
        reply_to: Some("123"),
        silent: false,
        media: None,
    };
    let id = m.send_message_with_options(opts).await.unwrap();
    assert!(id.starts_with("console:"));
}

#[tokio::test]
async fn console_set_typing_is_noop() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    m.set_typing("#channel", true).await.unwrap();
    m.set_typing("#channel", false).await.unwrap();
}

#[tokio::test]
async fn console_enqueue_direct_message() {
    let mut m = ConsoleMessenger::new("test".to_string());
    m.initialize().await.unwrap();
    m.enqueue(Message {
        id: "dm1".to_string(),
        sender: "bob".to_string(),
        content: "private message".to_string(),
        timestamp: 2000,
        channel: None,
        reply_to: None,
        media: None,
        is_direct: true,
        reactions: None,
    });
    let msgs = m.receive_messages().await.unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].is_direct);
    assert!(msgs[0].channel.is_none());
}
