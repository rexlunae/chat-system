//! IRC server implementation.

use crate::message::Message;
use crate::server::ChatServer;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// A basic IRC server that accepts connections and dispatches inbound `PRIVMSG`
/// lines to a handler callback.
pub struct IrcServer {
    address: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl IrcServer {
    /// Create a new [`IrcServer`] that will bind to `address` (e.g. `"127.0.0.1:6667"`).
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            shutdown_tx: None,
        }
    }
}

async fn handle_connection<F, Fut>(stream: tokio::net::TcpStream, handler: Arc<F>) -> Result<()>
where
    F: Fn(Message) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static,
{
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut nick = String::new();
    let mut user_seen = false;
    let mut registered = false;

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("NICK ") {
            nick = rest.trim().to_string();
        } else if line.starts_with("USER ") {
            user_seen = true;
        } else if line.starts_with("PING ") {
            let token = line.trim_start_matches("PING ");
            writer
                .write_all(format!("PONG {}\r\n", token).as_bytes())
                .await?;
        } else if let Some(rest) = line.strip_prefix("PRIVMSG ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let target = parts[0];
                let content = parts[1].trim_start_matches(':');
                let msg = Message {
                    id: format!("irc-{}", chrono::Utc::now().timestamp_millis()),
                    sender: nick.clone(),
                    content: content.to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    channel: Some(target.to_string()),
                    reply_to: None,
                    media: None,
                    is_direct: !target.starts_with('#'),
                    reactions: None,
                };
                if let Ok(Some(reply)) = handler(msg).await {
                    let response = format!(
                        ":server!server@localhost PRIVMSG {} :{}\r\n",
                        target, reply
                    );
                    writer.write_all(response.as_bytes()).await?;
                }
            }
        } else if line == "QUIT" || line.starts_with("QUIT ") {
            break;
        }

        if !registered && !nick.is_empty() && user_seen {
            writer
                .write_all(format!(":localhost 001 {} :Welcome\r\n", nick).as_bytes())
                .await?;
            registered = true;
        }
    }
    Ok(())
}

#[async_trait]
impl ChatServer for IrcServer {
    async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static,
    {
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        self.shutdown_tx = Some(tx);
        let handler = Arc::new(handler);
        let listener = TcpListener::bind(&self.address).await?;
        tracing::info!(address = %self.address, "IRC server listening");

        loop {
            tokio::select! {
                result = listener.accept() => {
                    let (stream, peer) = result?;
                    tracing::debug!(%peer, "IRC server: new connection");
                    let h = Arc::clone(&handler);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, h).await {
                            tracing::warn!("IRC server connection error: {e}");
                        }
                    });
                }
                _ = rx.changed() => {
                    if *rx.borrow() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn address(&self) -> &str {
        &self.address
    }

    async fn shutdown(&mut self) -> Result<()> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        Ok(())
    }
}
