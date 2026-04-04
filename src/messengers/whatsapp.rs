//! WhatsApp messenger stub (requires `whatsapp` feature).

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;

/// WhatsApp messenger stub.
pub struct WhatsAppMessenger {
    name: String,
    connected: bool,
}

impl WhatsAppMessenger {
    pub fn new(name: String) -> Self {
        Self {
            name,
            connected: false,
        }
    }
}

#[async_trait]
impl Messenger for WhatsAppMessenger {
    fn name(&self) -> &str {
        &self.name
    }
    fn messenger_type(&self) -> &str {
        "whatsapp"
    }
    async fn initialize(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }
    async fn send_message(&self, _recipient: &str, _content: &str) -> Result<String> {
        anyhow::bail!("WhatsApp messenger is a stub.")
    }
    async fn receive_messages(&self) -> Result<Vec<Message>> {
        Ok(Vec::new())
    }
    fn is_connected(&self) -> bool {
        self.connected
    }
    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}
