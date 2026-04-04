use anyhow::Result;
use async_trait::async_trait;
use chat_system::{ChatServer, Message};
use std::sync::{Arc, Mutex};

// ─── Mock implementation of ChatServer ───────────────────────────────────────

struct MockChatServer {
    address: String,
    /// Messages delivered to the handler during `run`.
    processed: Arc<Mutex<Vec<Message>>>,
}

impl MockChatServer {
    fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            processed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn processed_messages(&self) -> Vec<Message> {
        self.processed.lock().unwrap().clone()
    }
}

#[async_trait]
impl ChatServer for MockChatServer {
    /// Simulates receiving a single test message and passing it to the handler.
    async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static,
    {
        let msg = Message {
            id: "mock-1".to_string(),
            sender: "tester".to_string(),
            content: "hello server".to_string(),
            timestamp: 1_000,
            channel: Some("test-channel".to_string()),
            reply_to: None,
            media: None,
            is_direct: false,
        };
        self.processed.lock().unwrap().push(msg.clone());
        handler(msg).await?;
        Ok(())
    }

    fn address(&self) -> &str {
        &self.address
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn server_address() {
    let server = MockChatServer::new("127.0.0.1:8080");
    assert_eq!(server.address(), "127.0.0.1:8080");
}

#[tokio::test]
async fn server_address_arbitrary_string() {
    let server = MockChatServer::new("ws://example.com:9000/chat");
    assert_eq!(server.address(), "ws://example.com:9000/chat");
}

#[tokio::test]
async fn server_shutdown_is_ok() {
    let mut server = MockChatServer::new("127.0.0.1:0");
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn server_run_invokes_handler_with_message() {
    let mut server = MockChatServer::new("127.0.0.1:0");
    let received: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    server
        .run(move |msg| {
            let received = received_clone.clone();
            async move {
                received.lock().unwrap().push(msg);
                Ok(None)
            }
        })
        .await
        .unwrap();

    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].id, "mock-1");
    assert_eq!(msgs[0].sender, "tester");
    assert_eq!(msgs[0].content, "hello server");
    assert_eq!(msgs[0].channel, Some("test-channel".to_string()));
}

#[tokio::test]
async fn server_run_handler_can_return_reply() {
    let mut server = MockChatServer::new("127.0.0.1:0");
    let reply_seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let reply_clone = reply_seen.clone();

    server
        .run(move |_msg| {
            let reply_seen = reply_clone.clone();
            async move {
                let reply = Some("pong".to_string());
                *reply_seen.lock().unwrap() = reply.clone();
                Ok(reply)
            }
        })
        .await
        .unwrap();

    let reply = reply_seen.lock().unwrap();
    assert_eq!(*reply, Some("pong".to_string()));
}

#[tokio::test]
async fn server_run_records_processed_message_internally() {
    let mut server = MockChatServer::new("127.0.0.1:0");

    server.run(|_msg| async move { Ok(None) }).await.unwrap();

    let msgs = server.processed_messages();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "hello server");
}

#[tokio::test]
async fn server_run_handler_error_propagates() {
    let mut server = MockChatServer::new("127.0.0.1:0");

    let result = server
        .run(|_msg| async move { Err(anyhow::anyhow!("handler error")) })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("handler error"));
}

#[tokio::test]
async fn server_run_followed_by_shutdown() {
    let mut server = MockChatServer::new("127.0.0.1:0");

    server.run(|_msg| async move { Ok(None) }).await.unwrap();

    server.shutdown().await.unwrap();
}
