//! iMessage messenger — macOS-only stub using AppleScript.

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;

pub struct IMessageMessenger {
    name: String,
    connected: bool,
}

impl IMessageMessenger {
    pub fn new(name: String) -> Self {
        Self {
            name,
            connected: false,
        }
    }
}

#[async_trait]
impl Messenger for IMessageMessenger {
    fn name(&self) -> &str {
        &self.name
    }

    fn messenger_type(&self) -> &str {
        "imessage"
    }

    async fn initialize(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            self.connected = true;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            anyhow::bail!("iMessage is only supported on macOS");
        }
    }

    async fn send_message(&self, recipient: &str, content: &str) -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                r#"tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "{}" of targetService
    send "{}" to targetBuddy
end tell"#,
                recipient, content
            );
            let output = tokio::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
                .await?;
            if output.status.success() {
                Ok(format!(
                    "imessage:{}",
                    chrono::Utc::now().timestamp_millis()
                ))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("iMessage AppleScript failed: {}", stderr);
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (recipient, content);
            anyhow::bail!("iMessage is only supported on macOS");
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
