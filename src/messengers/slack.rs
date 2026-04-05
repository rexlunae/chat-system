//! Slack messenger — Web API implementation.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct SlackMessenger {
    name: String,
    token: String,
    client: Client,
    connected: bool,
}

impl SlackMessenger {
    pub fn new(name: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
            client: Client::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl Messenger for SlackMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "slack"
    }

    async fn initialize(&mut self) -> Result<()> {
        let resp = self
            .client
            .get("https://slack.com/api/auth.test")
            .bearer_auth(&self.token)
            .send()
            .await?;

        let data: Value = resp.json().await?;
        if data["ok"].as_bool().unwrap_or(false) {
            self.connected = true;
            Ok(())
        } else {
            anyhow::bail!("Slack auth.test failed: {:?}", data);
        }
    }

    async fn send_message(&self, channel: &str, text: &str) -> Result<String> {
        let resp = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.token)
            .json(&json!({
                "channel": channel,
                "text": text,
            }))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        if data["ok"].as_bool().unwrap_or(false) {
            let ts = data["ts"].as_str().unwrap_or("").to_string();
            Ok(ts)
        } else {
            anyhow::bail!("Slack chat.postMessage failed: {:?}", data);
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
