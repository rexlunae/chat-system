//! Core message types.

use serde::{Deserialize, Serialize};

/// The kind of a message, allowing callers to distinguish between regular
/// text messages and system events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// A regular text message.
    Text,
    /// A system/service message (join, leave, topic change, etc.).
    System,
    /// The message was edited (content reflects the latest version).
    Edit,
    /// The message was deleted (content may be empty).
    Delete,
    /// The message contains a media attachment (image, file, etc.).
    Media,
    /// An action / `/me` message (IRC ACTION, Slack `/me`).
    Action,
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::Text
    }
}

/// A single emoji reaction attached to a message, with an aggregate count and
/// the list of user IDs who reacted.
///
/// `user_ids` may be empty on platforms that only expose aggregate counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    /// The emoji (Unicode character or platform-specific shortcode).
    pub emoji: String,
    /// Number of users who added this reaction.
    pub count: u32,
    /// IDs of the users who reacted (may be empty if the platform does not expose them).
    #[serde(default)]
    pub user_ids: Vec<String>,
}

/// A message received from or sent to a chat platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    #[serde(default)]
    pub channel: Option<String>,
    /// ID of the message this is replying to, if any.
    #[serde(default)]
    pub reply_to: Option<String>,
    /// Thread / conversation ID, when the platform distinguishes threads from
    /// the main channel timeline (e.g. Slack, Discord, Matrix).
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub media: Option<Vec<MediaAttachment>>,
    #[serde(default)]
    pub is_direct: bool,
    /// The kind of this message.
    #[serde(default)]
    pub message_type: MessageType,
    /// When the message was last edited (Unix timestamp), if applicable.
    #[serde(default)]
    pub edited_timestamp: Option<i64>,
    /// Reactions attached to this message (populated when receiving messages on
    /// platforms that expose them; `None` means unknown / not fetched).
    #[serde(default)]
    pub reactions: Option<Vec<Reaction>>,
}

/// A media attachment in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    pub url: Option<String>,
    pub path: Option<String>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    /// File size in bytes, when known.
    #[serde(default)]
    pub size: Option<u64>,
}

/// Options for sending a message with additional metadata.
#[derive(Debug, Default)]
pub struct SendOptions<'a> {
    pub recipient: &'a str,
    pub content: &'a str,
    pub reply_to: Option<&'a str>,
    /// Thread ID to send into (platforms that support threading).
    pub thread_id: Option<&'a str>,
    pub silent: bool,
    pub media: Option<&'a str>,
}
