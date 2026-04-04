//! Discord messenger — REST API implementation.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct DiscordMessenger {
    name: String,
    token: String,
    client: Client,
    connected: bool,
}

impl DiscordMessenger {
    pub fn new(name: String, token: String) -> Self {
        Self {
            name,
            token,
            client: Client::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl Messenger for DiscordMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "discord"
    }

    async fn initialize(&mut self) -> Result<()> {
        // Verify the token by fetching the bot user
        let resp = self
            .client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await?;

        if resp.status().is_success() {
            self.connected = true;
            Ok(())
        } else {
            anyhow::bail!("Discord auth failed: {}", resp.status());
        }
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> Result<String> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );
        let body = json!({ "content": content });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            let data: Value = resp.json().await?;
            let id = data["id"].as_str().unwrap_or("").to_string();
            Ok(id)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Discord send_message failed {}: {}", status, text);
        }
    }

    async fn receive_messages(&self) -> Result<Vec<Message>> {
        // REST-only: no gateway. Return empty for now.
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
