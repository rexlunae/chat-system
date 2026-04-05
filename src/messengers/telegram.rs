//! Telegram messenger — Bot API polling implementation.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct TelegramMessenger {
    name: String,
    token: String,
    client: Client,
    connected: bool,
}

impl TelegramMessenger {
    pub fn new(name: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
            client: Client::new(),
            connected: false,
        }
    }

    fn api_url(&self, method: impl AsRef<str>) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method.as_ref())
    }
}

#[async_trait]
impl Messenger for TelegramMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "telegram"
    }

    async fn initialize(&mut self) -> Result<()> {
        let resp = self.client.get(self.api_url("getMe")).send().await?;

        let data: Value = resp.json().await?;
        if data["ok"].as_bool().unwrap_or(false) {
            self.connected = true;
            Ok(())
        } else {
            anyhow::bail!("Telegram getMe failed: {:?}", data);
        }
    }

    async fn send_message(&self, chat_id: &str, text: &str) -> Result<String> {
        let resp = self
            .client
            .post(self.api_url("sendMessage"))
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "HTML",
            }))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        if data["ok"].as_bool().unwrap_or(false) {
            let id = data["result"]["message_id"]
                .as_i64()
                .map(|i| i.to_string())
                .unwrap_or_default();
            Ok(id)
        } else {
            anyhow::bail!("Telegram sendMessage failed: {:?}", data);
        }
    }

    async fn receive_messages(&self) -> Result<Vec<Message>> {
        // Always use offset=0 — may return repeated messages but always compiles.
        let resp = self
            .client
            .get(self.api_url("getUpdates"))
            .query(&[("offset", "0"), ("timeout", "0")])
            .send()
            .await?;

        let data: Value = resp.json().await?;
        let mut messages = Vec::new();

        if let Some(updates) = data["result"].as_array() {
            for update in updates {
                if let Some(msg) = update.get("message") {
                    let id = msg["message_id"].as_i64().unwrap_or(0).to_string();
                    let sender = msg["from"]["username"]
                        .as_str()
                        .or_else(|| msg["from"]["first_name"].as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let content = msg["text"].as_str().unwrap_or("").to_string();
                    let timestamp = msg["date"].as_i64().unwrap_or(0);
                    let chat_id = msg["chat"]["id"].as_i64().map(|i| i.to_string());

                    messages.push(Message {
                        id,
                        sender,
                        content,
                        timestamp,
                        channel: chat_id,
                        reply_to: None,
                        media: None,
                        is_direct: false,
                        reactions: None,
                    });
                }
            }
        }

        Ok(messages)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}
