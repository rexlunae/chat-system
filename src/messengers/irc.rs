//! IRC messenger — raw TCP implementation using tokio.

use crate::{Message, Messenger};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct IrcMessenger {
    name: String,
    server: String,
    port: u16,
    nick: String,
    channels: Vec<String>,
    use_tls: bool,
    writer: Option<Arc<Mutex<tokio::io::WriteHalf<TcpStream>>>>,
    reader: Option<Arc<Mutex<BufReader<tokio::io::ReadHalf<TcpStream>>>>>,
    connected: bool,
}

impl IrcMessenger {
    pub fn new(name: String, server: String, port: u16, nick: String) -> Self {
        Self {
            name,
            server,
            port,
            nick,
            channels: Vec::new(),
            use_tls: false,
            writer: None,
            reader: None,
            connected: false,
        }
    }

    pub fn with_channels(mut self, channels: Vec<String>) -> Self {
        self.channels = channels;
        self
    }

    pub fn with_tls(mut self, tls: bool) -> Self {
        self.use_tls = tls;
        self
    }

    async fn send_raw(&self, line: &str) -> Result<()> {
        if let Some(writer) = &self.writer {
            let mut w = writer.lock().await;
            w.write_all(format!("{}\r\n", line).as_bytes()).await?;
        }
        Ok(())
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
        let stream = TcpStream::connect(&addr)
            .await
            .with_context(|| format!("Failed to connect to IRC server {}", addr))?;

        let (read_half, write_half) = tokio::io::split(stream);
        self.writer = Some(Arc::new(Mutex::new(write_half)));
        self.reader = Some(Arc::new(Mutex::new(BufReader::new(read_half))));

        // Register with the server
        self.send_raw(&format!("NICK {}", self.nick)).await?;
        self.send_raw(&format!("USER {} 0 * :{}", self.nick, self.nick)).await?;

        // Wait for welcome (001) or error
        if let Some(reader) = &self.reader {
            let mut r = reader.lock().await;
            let mut line = String::new();
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                if tokio::time::Instant::now() > deadline {
                    break;
                }
                line.clear();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    r.read_line(&mut line),
                )
                .await
                {
                    Ok(Ok(0)) | Err(_) => break,
                    Ok(Ok(_)) => {
                        let trimmed = line.trim();
                        if trimmed.contains("001") {
                            break;
                        }
                        if trimmed.starts_with("PING ") {
                            let token = trimmed.trim_start_matches("PING ").to_string();
                            drop(r);
                            self.send_raw(&format!("PONG {}", token)).await?;
                            r = reader.lock().await;
                        }
                    }
                    Ok(Err(e)) => return Err(e.into()),
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
        self.send_raw(&format!("PRIVMSG {} :{}", recipient, content)).await?;
        Ok(format!("irc:{}:{}", recipient, content))
    }

    async fn receive_messages(&self) -> Result<Vec<Message>> {
        let mut messages = Vec::new();

        if let Some(reader) = &self.reader {
            let mut r = reader.lock().await;
            let mut line = String::new();

            loop {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    r.read_line(&mut line),
                )
                .await
                {
                    Ok(Ok(0)) | Err(_) => break,
                    Ok(Ok(_)) => {
                        let trimmed = line.trim().to_string();
                        if trimmed.contains("PRIVMSG") {
                            // :nick!user@host PRIVMSG #channel :message
                            let parts: Vec<&str> = trimmed.splitn(4, ' ').collect();
                            if parts.len() >= 4 {
                                let sender = parts[0]
                                    .trim_start_matches(':')
                                    .split('!')
                                    .next()
                                    .unwrap_or("unknown")
                                    .to_string();
                                let channel = parts[2].to_string();
                                let content = parts[3].trim_start_matches(':').to_string();
                                messages.push(Message {
                                    id: format!("irc-{}", chrono::Utc::now().timestamp_millis()),
                                    sender,
                                    content,
                                    timestamp: chrono::Utc::now().timestamp(),
                                    channel: Some(channel),
                                    reply_to: None,
                                    media: None,
                                    is_direct: false,
                                });
                            }
                        }
                        line.clear();
                    }
                    Ok(Err(e)) => return Err(e.into()),
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
        self.writer = None;
        self.reader = None;
        Ok(())
    }
}
