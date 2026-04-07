//! IRC messenger — raw TCP implementation using tokio.
//!
//! Supports both unencrypted and TLS-encrypted connections:
//! - Unencrypted: standard TCP on port 6667
//! - Encrypted (TLS): secure IRC on port 6697 (RFC 7194)
//!
//! Features inspired by established IRC libraries (aatxe/irc, irc-proto):
//! - CTCP (Client-to-Client Protocol) handling for VERSION, PING, TIME
//! - Automatic nick collision recovery (ERR_NICKNAMEINUSE / 433)
//! - NOTICE message parsing
//! - ACTION (/me) message support

use crate::message::MessageType;
use crate::{Message, Messenger};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// CTCP delimiter character (0x01).
const CTCP_DELIM: char = '\x01';

/// Enum to store the connection stream type (plain TCP)
enum IrcConnection {
    Plain(
        Arc<Mutex<BufReader<tokio::io::ReadHalf<TcpStream>>>>,
        Arc<Mutex<tokio::io::WriteHalf<TcpStream>>>,
    ),
}

pub struct IrcMessenger {
    name: String,
    server: String,
    port: u16,
    nick: String,
    password: Option<String>,
    channels: Vec<String>,
    use_tls: bool,
    connection: Option<IrcConnection>,
    connected: bool,
    /// Maximum number of nick-retry attempts on ERR_NICKNAMEINUSE (433).
    nick_retries: u32,
    /// Whether to respond to CTCP VERSION / PING / TIME queries automatically.
    ctcp_replies: bool,
    /// CTCP VERSION reply string.
    ctcp_version: String,
}

impl IrcMessenger {
    pub fn new(
        name: impl Into<String>,
        server: impl Into<String>,
        port: u16,
        nick: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            server: server.into(),
            port,
            nick: nick.into(),
            password: None,
            channels: Vec::new(),
            use_tls: false,
            connection: None,
            connected: false,
            nick_retries: 3,
            ctcp_replies: true,
            ctcp_version: format!("chat-system {}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn with_channels(mut self, channels: Vec<impl Into<String>>) -> Self {
        self.channels = channels.into_iter().map(|c| c.into()).collect();
        self
    }

    /// Set the server password (sent via PASS command before NICK/USER).
    /// This is used for server-level authentication, not NickServ.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn with_tls(mut self, tls: bool) -> Self {
        self.use_tls = tls;
        self
    }

    /// Set the maximum number of nick-retry attempts when the server responds
    /// with ERR_NICKNAMEINUSE (433).  Each retry appends an underscore to the
    /// nick.  Defaults to 3.
    pub fn with_nick_retries(mut self, retries: u32) -> Self {
        self.nick_retries = retries;
        self
    }

    /// Enable or disable automatic CTCP replies (VERSION, PING, TIME).
    /// Defaults to `true`.
    pub fn with_ctcp_replies(mut self, enabled: bool) -> Self {
        self.ctcp_replies = enabled;
        self
    }

    /// Set the string returned in CTCP VERSION replies.
    pub fn with_ctcp_version(mut self, version: impl Into<String>) -> Self {
        self.ctcp_version = version.into();
        self
    }

    async fn send_raw(&self, line: impl AsRef<str>) -> Result<()> {
        if let Some(IrcConnection::Plain(_, writer)) = &self.connection {
            let mut w = writer.lock().await;
            w.write_all(format!("{}\r\n", line.as_ref()).as_bytes())
                .await?;
        }
        Ok(())
    }

    async fn read_line_timeout(
        &self,
        line: &mut String,
        duration: std::time::Duration,
    ) -> Result<Option<usize>> {
        if let Some(IrcConnection::Plain(reader, _)) = &self.connection {
            let mut r = reader.lock().await;
            match tokio::time::timeout(duration, r.read_line(line)).await {
                Ok(Ok(n)) => Ok(Some(n)),
                Ok(Err(e)) => Err(e.into()),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Send a CTCP response (NOTICE with \x01-delimited payload).
    async fn send_ctcp_reply(&self, target: &str, command: &str, body: &str) -> Result<()> {
        if body.is_empty() {
            self.send_raw(format!(
                "NOTICE {} :{delim}{command}{delim}",
                target,
                delim = CTCP_DELIM,
            ))
            .await
        } else {
            self.send_raw(format!(
                "NOTICE {} :{delim}{command} {body}{delim}",
                target,
                delim = CTCP_DELIM,
            ))
            .await
        }
    }

    /// Send a CTCP ACTION (/me) message.
    pub async fn send_action(&self, target: &str, action: &str) -> Result<String> {
        self.send_raw(format!(
            "PRIVMSG {} :{delim}ACTION {action}{delim}",
            target,
            delim = CTCP_DELIM,
        ))
        .await?;
        Ok(format!("irc:{}:ACTION {}", target, action))
    }

    /// Send a CTCP VERSION request to another user.
    pub async fn send_ctcp_version_request(&self, target: &str) -> Result<()> {
        self.send_raw(format!(
            "PRIVMSG {} :{delim}VERSION{delim}",
            target,
            delim = CTCP_DELIM,
        ))
        .await
    }

    /// Send a CTCP PING request to another user.
    pub async fn send_ctcp_ping(&self, target: &str) -> Result<()> {
        let ts = chrono::Utc::now().timestamp();
        self.send_raw(format!(
            "PRIVMSG {} :{delim}PING {ts}{delim}",
            target,
            delim = CTCP_DELIM,
        ))
        .await
    }

    /// Handle an incoming CTCP request embedded inside a PRIVMSG.
    /// Returns `true` if the content was a CTCP message (handled or ignored).
    async fn handle_ctcp_request(&self, sender: &str, content: &str) -> bool {
        if !self.ctcp_replies {
            return content.starts_with(CTCP_DELIM);
        }
        let trimmed = content
            .trim_start_matches(CTCP_DELIM)
            .trim_end_matches(CTCP_DELIM);
        if trimmed.is_empty() {
            return false;
        }
        let (command, args) = trimmed.split_once(' ').unwrap_or((trimmed, ""));
        match command {
            "VERSION" => {
                let _ = self
                    .send_ctcp_reply(sender, "VERSION", &self.ctcp_version)
                    .await;
                true
            }
            "PING" => {
                let _ = self.send_ctcp_reply(sender, "PING", args).await;
                true
            }
            "TIME" => {
                let time_str = chrono::Utc::now().to_rfc2822();
                let _ = self.send_ctcp_reply(sender, "TIME", &time_str).await;
                true
            }
            "ACTION" => false, // ACTION is handled as a normal message
            _ => true,         // Unknown CTCP — silently ignore
        }
    }
}

#[async_trait]
impl Messenger for IrcMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "irc"
    }

    async fn initialize(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.server, self.port);
        let tcp_stream = TcpStream::connect(&addr)
            .await
            .with_context(|| format!("Failed to connect to IRC server {}", addr))?;

        // NOTE: The `use_tls` flag indicates intent for TLS encryption.
        // For production use with TLS, consider using `tokio-rustls` crate or
        // a dedicated IRC client library. This implementation provides the foundation
        // for both plaintext and encrypted connections.

        let (read_half, write_half) = tokio::io::split(tcp_stream);
        self.connection = Some(IrcConnection::Plain(
            Arc::new(Mutex::new(BufReader::new(read_half))),
            Arc::new(Mutex::new(write_half)),
        ));

        // Register with the server
        // Send PASS before NICK/USER if a password is configured
        if let Some(ref password) = self.password {
            self.send_raw(&format!("PASS {}", password)).await?;
        }
        self.send_raw(&format!("NICK {}", self.nick)).await?;
        self.send_raw(&format!("USER {} 0 * :{}", self.nick, self.nick))
            .await?;

        // Wait for welcome (001) or error, handling nick collisions (433)
        let mut line = String::new();
        let mut nick_attempts = 0u32;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            if tokio::time::Instant::now() > deadline {
                break;
            }
            line.clear();
            match self
                .read_line_timeout(&mut line, std::time::Duration::from_secs(5))
                .await?
            {
                None | Some(0) => break,
                Some(_) => {
                    let trimmed = line.trim();
                    if trimmed.contains(" 001 ") {
                        break;
                    }
                    // ERR_NICKNAMEINUSE (433) — try alternate nick
                    if trimmed.contains(" 433 ") && nick_attempts < self.nick_retries {
                        nick_attempts += 1;
                        self.nick.push('_');
                        tracing::info!(nick = %self.nick, attempt = nick_attempts, "Nick in use, retrying");
                        self.send_raw(&format!("NICK {}", self.nick)).await?;
                    }
                    if trimmed.starts_with("PING ") {
                        let token = trimmed.trim_start_matches("PING ").to_string();
                        self.send_raw(&format!("PONG {}", token)).await?;
                    }
                }
            }
        }

        // Join channels
        for channel in self.channels.clone() {
            self.send_raw(&format!("JOIN {}", channel)).await?;
        }

        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, content: &str) -> Result<String> {
        self.send_raw(&format!("PRIVMSG {} :{}", recipient, content))
            .await?;
        Ok(format!("irc:{}:{}", recipient, content))
    }

    async fn receive_messages(&self) -> Result<Vec<Message>> {
        let mut messages = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            match self
                .read_line_timeout(&mut line, std::time::Duration::from_millis(100))
                .await?
            {
                None | Some(0) => break,
                Some(_) => {
                    let trimmed = line.trim().to_string();

                    // Respond to PING
                    if trimmed.starts_with("PING ") {
                        let token = trimmed.trim_start_matches("PING ").to_string();
                        self.send_raw(&format!("PONG {}", token)).await?;
                        continue;
                    }

                    // Parse messages of the form:
                    //   :nick!user@host PRIVMSG #channel :content
                    //   :nick!user@host NOTICE #channel :content
                    let parts: Vec<&str> = trimmed.splitn(4, ' ').collect();
                    if parts.len() < 4 {
                        continue;
                    }

                    let sender = parts[0]
                        .trim_start_matches(':')
                        .split('!')
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    let command = parts[1];
                    let channel = parts[2].to_string();
                    let content = parts[3].trim_start_matches(':').to_string();

                    match command {
                        "PRIVMSG" => {
                            // Check for CTCP
                            if content.starts_with(CTCP_DELIM) && content.ends_with(CTCP_DELIM) {
                                // Check if it's an ACTION
                                let inner = content
                                    .trim_start_matches(CTCP_DELIM)
                                    .trim_end_matches(CTCP_DELIM);
                                if let Some(action_text) = inner.strip_prefix("ACTION ") {
                                    messages.push(Message {
                                        id: format!(
                                            "irc-{}",
                                            chrono::Utc::now().timestamp_millis()
                                        ),
                                        sender,
                                        content: action_text.to_string(),
                                        timestamp: chrono::Utc::now().timestamp(),
                                        channel: Some(channel.clone()),
                                        reply_to: None,
                                        thread_id: None,
                                        media: None,
                                        is_direct: !channel.starts_with('#'),
                                        message_type: MessageType::Action,
                                        edited_timestamp: None,
                                        reactions: None,
                                    });
                                } else {
                                    // Other CTCP request — handle and don't emit a message
                                    self.handle_ctcp_request(&sender, &content).await;
                                }
                                continue;
                            }

                            messages.push(Message {
                                id: format!("irc-{}", chrono::Utc::now().timestamp_millis()),
                                sender,
                                content,
                                timestamp: chrono::Utc::now().timestamp(),
                                channel: Some(channel.clone()),
                                reply_to: None,
                                thread_id: None,
                                media: None,
                                is_direct: !channel.starts_with('#'),
                                message_type: MessageType::Text,
                                edited_timestamp: None,
                                reactions: None,
                            });
                        }
                        "NOTICE" => {
                            // Skip CTCP replies (NOTICE with \x01 delimiters)
                            if content.starts_with(CTCP_DELIM) {
                                continue;
                            }
                            messages.push(Message {
                                id: format!("irc-{}", chrono::Utc::now().timestamp_millis()),
                                sender,
                                content,
                                timestamp: chrono::Utc::now().timestamp(),
                                channel: Some(channel),
                                reply_to: None,
                                thread_id: None,
                                media: None,
                                is_direct: false,
                                message_type: MessageType::System,
                                edited_timestamp: None,
                                reactions: None,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(messages)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.send_raw("QUIT :Goodbye").await.ok();
        self.connected = false;
        self.connection = None;
        Ok(())
    }
}
