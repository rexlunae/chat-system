//! IRC server implementation.

use crate::message::Message;
use crate::server::{ChatListener, ChatServer};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

// ── IrcListener ───────────────────────────────────────────────────────────────

/// A TCP listener that accepts incoming IRC connections and forwards them to an
/// [`IrcServer`] event loop.
///
/// Multiple `IrcListener` instances can be added to a single [`IrcServer`] so
/// that the server can accept connections on several ports (or with different
/// transport options such as TLS).
pub struct IrcListener {
    address: String,
    /// Shutdown sender stored after the listener task has been spawned.
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl IrcListener {
    /// Create a new [`IrcListener`] that will bind to `address` when the
    /// owning [`IrcServer`] is started.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            shutdown_tx: None,
        }
    }

    /// Bind the listener, spawn its accept loop, and wire accepted connections
    /// into `conn_tx`.
    ///
    /// Returns a [`tokio::sync::watch::Sender`] that the caller can use to
    /// signal the accept loop to stop.  This method is called internally by
    /// [`IrcServer::run`].
    pub(crate) async fn spawn(
        &mut self,
        conn_tx: tokio::sync::mpsc::Sender<tokio::net::TcpStream>,
    ) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let listener = TcpListener::bind(&self.address).await?;
        // Update the stored address to the actual bound address (useful when
        // port 0 was requested).
        self.address = listener.local_addr()?.to_string();
        tracing::info!(address = %self.address, "IRC listener bound");
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer)) => {
                                tracing::debug!(%peer, "IRC listener: new connection");
                                if conn_tx.send(stream).await.is_err() {
                                    // Server channel closed — stop accepting.
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("IRC listener accept error: {e}");
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl ChatListener for IrcListener {
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

// ── IrcServer ─────────────────────────────────────────────────────────────────

/// A basic IRC server that accepts connections from one or more listeners and
/// dispatches inbound `PRIVMSG` lines to a handler callback.
///
/// Use [`IrcServer::add_listener`] to attach additional listeners before
/// calling [`ChatServer::run`]:
///
/// ```rust,no_run
/// use chat_system::servers::IrcServer;
/// use chat_system::IrcListener;
///
/// let mut server = IrcServer::new("127.0.0.1:6667");
/// server.add_listener(IrcListener::new("127.0.0.1:6668"));
/// ```
pub struct IrcServer {
    listeners: Vec<IrcListener>,
}

impl IrcServer {
    /// Create a new [`IrcServer`] with a single listener bound to `address`.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            listeners: vec![IrcListener::new(address)],
        }
    }

    /// Add an extra [`IrcListener`] to this server.
    ///
    /// Additional listeners must be added *before* calling
    /// [`ChatServer::run`]; listeners added after `run` has been called are
    /// ignored.
    pub fn add_listener(&mut self, listener: IrcListener) -> &mut Self {
        self.listeners.push(listener);
        self
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
                    let response =
                        format!(":server!server@localhost PRIVMSG {} :{}\r\n", target, reply);
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
        let handler = Arc::new(handler);

        // Channel through which every listener forwards accepted streams.
        let (conn_tx, mut conn_rx) = tokio::sync::mpsc::channel::<tokio::net::TcpStream>(128);

        // Spawn all listeners.  Each sends accepted streams into `conn_tx`.
        for listener in &mut self.listeners {
            listener.spawn(conn_tx.clone()).await?;
        }

        // Drop our own clone of conn_tx so that conn_rx returns None once all
        // listener tasks have exited (or been shut down).
        drop(conn_tx);

        // Event loop: receive connections from any listener and handle them.
        while let Some(stream) = conn_rx.recv().await {
            let h = Arc::clone(&handler);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, h).await {
                    tracing::warn!("IRC server connection error: {e}");
                }
            });
        }

        Ok(())
    }

    fn address(&self) -> &str {
        self.listeners
            .first()
            .map(|l| l.address())
            .unwrap_or_default()
    }

    fn addresses(&self) -> Vec<&str> {
        self.listeners.iter().map(|l| l.address()).collect()
    }

    async fn shutdown(&mut self) -> Result<()> {
        for listener in &mut self.listeners {
            listener.shutdown().await?;
        }
        Ok(())
    }
}
