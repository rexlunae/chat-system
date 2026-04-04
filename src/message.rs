//! Core message types.

use serde::{Deserialize, Serialize};

/// A message received from or sent to a chat platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub media: Option<Vec<MediaAttachment>>,
    #[serde(default)]
    pub is_direct: bool,
}

/// A media attachment in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    pub url: Option<String>,
    pub path: Option<String>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
}

/// Options for sending a message with additional metadata.
#[derive(Debug, Default)]
pub struct SendOptions<'a> {
    pub recipient: &'a str,
    pub content: &'a str,
    pub reply_to: Option<&'a str>,
    pub silent: bool,
    pub media: Option<&'a str>,
}
