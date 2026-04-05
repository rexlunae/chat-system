//! The [`Messenger`] trait and [`MessengerManager`].

use crate::message::{Message, SendOptions};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The presence/availability status of a messenger account or bot.
///
/// Not every platform supports every variant; unsupported values fall back to
/// the closest equivalent or are silently ignored via the default no-op
/// implementation of [`Messenger::set_status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    /// Fully available and accepting messages.
    Online,
    /// Temporarily away (e.g. idle, away message set).
    Away,
    /// Do-not-disturb / busy — notifications may be suppressed.
    Busy,
    /// Signed in but appearing as offline to other users.
    Invisible,
    /// Fully offline / disconnected.
    Offline,
}

/// A unified interface for chat platform clients.
#[async_trait]
pub trait Messenger: Send + Sync {
    fn name(&self) -> &str;
    fn messenger_type(&self) -> &str;
    async fn initialize(&mut self) -> Result<()>;
    async fn send_message(&self, recipient: &str, content: &str) -> Result<String>;
    async fn send_message_with_options(&self, opts: SendOptions<'_>) -> Result<String> {
        self.send_message(opts.recipient, opts.content).await
    }
    async fn receive_messages(&self) -> Result<Vec<Message>>;
    fn is_connected(&self) -> bool;
    async fn disconnect(&mut self) -> Result<()>;
    async fn set_typing(&self, _channel: &str, _typing: bool) -> Result<()> {
        Ok(())
    }
    /// Set the bot's own presence/availability status.
    ///
    /// Platforms that do not support a particular [`PresenceStatus`] value, or
    /// that have no presence API at all, may ignore this call.  The default
    /// implementation is a no-op so that existing messenger implementations
    /// are unaffected.
    async fn set_status(&self, _status: PresenceStatus) -> Result<()> {
        Ok(())
    }
}

/// Manages multiple [`Messenger`] instances.
pub struct MessengerManager {
    messengers: Vec<Box<dyn Messenger>>,
}

impl MessengerManager {
    pub fn new() -> Self {
        Self {
            messengers: Vec::new(),
        }
    }

    pub fn add(&mut self, messenger: Box<dyn Messenger>) {
        self.messengers.push(messenger);
    }

    pub async fn initialize_all(&mut self) -> Result<()> {
        for m in &mut self.messengers {
            m.initialize().await?;
        }
        Ok(())
    }

    pub async fn disconnect_all(&mut self) -> Result<()> {
        for m in &mut self.messengers {
            m.disconnect().await?;
        }
        Ok(())
    }

    pub async fn receive_all(&self) -> Result<Vec<Message>> {
        let mut all = Vec::new();
        for m in &self.messengers {
            match m.receive_messages().await {
                Ok(mut msgs) => all.append(&mut msgs),
                Err(e) => tracing::warn!(messenger = %m.name(), "receive error: {e}"),
            }
        }
        Ok(all)
    }

    pub async fn broadcast(&self, recipient: impl AsRef<str>, content: impl AsRef<str>) -> Vec<Result<String>> {
        let mut results = Vec::new();
        for m in &self.messengers {
            results.push(m.send_message(recipient.as_ref(), content.as_ref()).await);
        }
        results
    }

    pub fn messengers(&self) -> &[Box<dyn Messenger>] {
        &self.messengers
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&dyn Messenger> {
        self.messengers
            .iter()
            .find(|m| m.name() == name.as_ref())
            .map(|b| b.as_ref())
    }
}

impl Default for MessengerManager {
    fn default() -> Self {
        Self::new()
    }
}
