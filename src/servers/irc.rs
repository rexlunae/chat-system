//! IRC listener implementation.
//!
//! Implements a basic IRC server that handles connection registration,
//! PRIVMSG/NOTICE, PING/PONG, JOIN/PART, TOPIC, and the standard
//! RPL_WELCOME sequence (001–004).

use crate::message::{Message, MessageType};
use crate::server::{ChatListener, MessageHandler};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// CTCP delimiter character (0x01).
const CTCP_DELIM: char = '\x01';

// ── IrcListener ───────────────────────────────────────────────────────────────

/// A TCP listener that speaks the IRC protocol.
///
/// When started, it binds the configured address, accepts incoming connections,
/// parses IRC messages, invokes the message handler, and sends replies back in
/// IRC wire format.  Multiple `IrcListener` instances can be attached to a
/// single [`Server`](crate::server::Server) so that it is reachable on several
/// ports simultaneously.
///
/// ```rust,no_run
/// use chat_system::server::Server;
/// use chat_system::servers::IrcListener;
///
/// # #[tokio::main] async fn main() -> anyhow::Result<()> {
/// let mut server = Server::new("my-irc")
///     .add_listener(IrcListener::new("0.0.0.0:6667"))
///     .add_listener(IrcListener::new("0.0.0.0:6697"));
/// // server.run(handler).await?;
/// # Ok(()) }
/// ```
pub struct IrcListener {
    address: String,
    server_name: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl IrcListener {
    /// Create a new [`IrcListener`] that will bind to `address` (e.g.
    /// `"127.0.0.1:6667"`).
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            server_name: "localhost".to_string(),
            shutdown_tx: None,
        }
    }

    /// Set a custom server name used in the RPL_WELCOME sequence.
    pub fn with_server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }
}

/// Send the standard IRC welcome sequence (RPL 001–004) to a newly registered
/// client.
async fn send_welcome(
    writer: &mut (impl tokio::io::AsyncWrite + Unpin),
    server_name: &str,
    nick: &str,
) -> Result<()> {
    let lines = [
        format!(":{server_name} 001 {nick} :Welcome to the Internet Relay Network {nick}\r\n"),
        format!(":{server_name} 002 {nick} :Your host is {server_name}, running chat-system\r\n"),
        format!(":{server_name} 003 {nick} :This server was created with chat-system\r\n"),
        format!(":{server_name} 004 {nick} {server_name} chat-system o o\r\n"),
    ];
    for line in &lines {
        writer.write_all(line.as_bytes()).await?;
    }
    Ok(())
}

/// Handle a single IRC connection: perform the handshake, parse `PRIVMSG`
/// lines, invoke the handler, and write replies.
///
/// Generic over the stream type so it can be used with both plain TCP and TLS
/// connections.
#[allow(dead_code)] // used by TlsIrcListener behind `tls` feature gate
pub(super) async fn handle_connection(
    stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    handler: MessageHandler,
) -> Result<()> {
    handle_connection_with_name(stream, handler, "localhost").await
}

/// Like [`handle_connection`], but allows specifying a custom server name for
/// the welcome sequence.
pub(super) async fn handle_connection_with_name(
    stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    handler: MessageHandler,
    server_name: &str,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
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

                // Detect CTCP ACTION and map to MessageType::Action
                let (msg_content, msg_type) =
                    if content.starts_with(CTCP_DELIM) && content.ends_with(CTCP_DELIM) {
                        let inner = content
                            .trim_start_matches(CTCP_DELIM)
                            .trim_end_matches(CTCP_DELIM);
                        if let Some(action_text) = inner.strip_prefix("ACTION ") {
                            (action_text.to_string(), MessageType::Action)
                        } else {
                            // Other CTCP in server context — skip
                            continue;
                        }
                    } else {
                        (content.to_string(), MessageType::Text)
                    };

                let msg = Message {
                    id: format!("irc-{}", chrono::Utc::now().timestamp_millis()),
                    sender: nick.clone(),
                    content: msg_content,
                    timestamp: chrono::Utc::now().timestamp(),
                    channel: Some(target.to_string()),
                    reply_to: None,
                    thread_id: None,
                    media: None,
                    is_direct: !target.starts_with('#'),
                    message_type: msg_type,
                    edited_timestamp: None,
                    reactions: None,
                };
                if let Ok(Some(reply)) = handler(msg).await {
                    let response = format!(
                        ":{server_name}!{server_name}@{server_name} PRIVMSG {} :{}\r\n",
                        target, reply
                    );
                    writer.write_all(response.as_bytes()).await?;
                }
            }
        } else if let Some(rest) = line.strip_prefix("NOTICE ") {
            // Parse NOTICE similarly to PRIVMSG but deliver as System
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
                    thread_id: None,
                    media: None,
                    is_direct: !target.starts_with('#'),
                    message_type: MessageType::System,
                    edited_timestamp: None,
                    reactions: None,
                };
                // NOTICEs do not generate automatic replies per IRC spec
                let _ = handler(msg).await;
            }
        } else if let Some(rest) = line.strip_prefix("JOIN ") {
            let channel = rest.trim().trim_start_matches(':');
            // Echo JOIN back to the client
            writer
                .write_all(format!(":{nick}!{nick}@{server_name} JOIN {channel}\r\n").as_bytes())
                .await?;
        } else if line.starts_with("PART ") || line.starts_with("TOPIC ") {
            // Acknowledge silently for now
        } else if line == "QUIT" || line.starts_with("QUIT ") {
            break;
        }

        if !registered && !nick.is_empty() && user_seen {
            send_welcome(&mut writer, server_name, &nick).await?;
            registered = true;
        }
    }
    Ok(())
}

#[async_trait]
impl ChatListener for IrcListener {
    fn address(&self) -> &str {
        &self.address
    }

    fn protocol(&self) -> &str {
        "irc"
    }

    async fn start(
        &mut self,
        handler: MessageHandler,
        alive: tokio::sync::mpsc::Sender<()>,
    ) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let listener = TcpListener::bind(&self.address).await?;
        // Update to the actual bound address (useful when port 0 is requested).
        self.address = listener.local_addr()?.to_string();
        tracing::info!(address = %self.address, "IRC listener bound");
        self.shutdown_tx = Some(shutdown_tx);
        let server_name = self.server_name.clone();

        tokio::spawn(async move {
            // Hold `alive` — when this task exits, the sender is dropped,
            // signalling the server that this listener has stopped.
            let _alive = alive;

            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer)) => {
                                tracing::debug!(%peer, "IRC listener: new connection");
                                let h = Arc::clone(&handler);
                                let sn = server_name.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection_with_name(stream, h, &sn).await {
                                        tracing::warn!("IRC connection error: {e}");
                                    }
                                });
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

    async fn shutdown(&mut self) -> Result<()> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        Ok(())
    }
}
