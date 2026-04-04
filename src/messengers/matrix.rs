//! Matrix messenger stub (requires `matrix` feature).

use crate::{Message, Messenger};
use anyhow::Result;
use async_trait::async_trait;

/// Matrix messenger stub. Full implementation pending matrix-sdk API stabilization.
pub struct MatrixMessenger {
    name: String,
    homeserver: String,
    username: String,
    password: String,
    connected: bool,
}

impl MatrixMessenger {
    pub fn new(name: String, homeserver: String, username: String, password: String) -> Self {
        Self {
            name,
            homeserver,
            username,
            password,
            connected: false,
        }
    }
}

#[async_trait]
impl Messenger for MatrixMessenger {
    fn name(&self) -> &str {
        &self.name
    }
    fn messenger_type(&self) -> &str {
        "matrix"
    }
    async fn initialize(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }
    async fn send_message(&self, _room: &str, _content: &str) -> Result<String> {
        anyhow::bail!("Matrix messenger is a stub. Enable the matrix feature and implement fully.")
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
