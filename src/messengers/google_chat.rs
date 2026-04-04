//! Google Chat messenger — Incoming Webhook implementation.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

pub struct GoogleChatMessenger {
    name: String,
    webhook_url: String,
    client: Client,
    connected: bool,
}

impl GoogleChatMessenger {
    pub fn new(name: String, webhook_url: String) -> Self {
        Self {
            name,
            webhook_url,
            client: Client::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl Messenger for GoogleChatMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "googlechat"
    }

    async fn initialize(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, _space: &str, content: &str) -> Result<String> {
        let body = json!({ "text": content });

        let resp = self.client.post(&self.webhook_url).json(&body).send().await?;

        if resp.status().is_success() {
            Ok(format!("googlechat:{}", chrono::Utc::now().timestamp_millis()))
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Google Chat webhook failed {}: {}", status, text);
        }
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
