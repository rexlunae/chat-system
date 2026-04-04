//! Microsoft Teams messenger — Incoming Webhook implementation.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

pub struct TeamsMessenger {
    name: String,
    webhook_url: String,
    client: Client,
    connected: bool,
}

impl TeamsMessenger {
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
impl Messenger for TeamsMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "msteams"
    }

    async fn initialize(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, _channel: &str, content: &str) -> Result<String> {
        let body = json!({
            "@type": "MessageCard",
            "@context": "https://schema.org/extensions",
            "text": content,
        });

        let resp = self
            .client
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(format!("teams:{}", chrono::Utc::now().timestamp_millis()))
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Teams webhook failed {}: {}", status, text);
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
