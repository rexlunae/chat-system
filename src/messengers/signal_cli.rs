//! Signal messenger — signal-cli subprocess wrapper.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;

pub struct SignalCliMessenger {
    name: String,
    phone_number: String,
    signal_cli_path: String,
    connected: bool,
}

impl SignalCliMessenger {
    pub fn new(name: impl Into<String>, phone_number: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            phone_number: phone_number.into(),
            signal_cli_path: "signal-cli".to_string(),
            connected: false,
        }
    }

    pub fn with_cli_path(mut self, path: impl Into<String>) -> Self {
        self.signal_cli_path = path.into();
        self
    }
}

#[async_trait]
impl Messenger for SignalCliMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "signal"
    }

    async fn initialize(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, content: &str) -> Result<String> {
        let output = tokio::process::Command::new(&self.signal_cli_path)
            .args(["-u", &self.phone_number, "send", "-m", content, recipient])
            .output()
            .await?;

        if output.status.success() {
            Ok(format!("signal:{}", chrono::Utc::now().timestamp_millis()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("signal-cli send failed: {}", stderr);
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
