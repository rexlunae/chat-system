//! # chat-system
//!
//! A multi-protocol async chat crate for Rust. Provides a unified interface
//! to IRC, Matrix, Discord, Telegram, Slack, Signal, WhatsApp, and more.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use chat_system::messengers::IrcMessenger;
//! use chat_system::Messenger;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut client = IrcMessenger::new(
//!         "my-bot".into(),
//!         "irc.libera.chat".into(),
//!         6697,
//!         "chat-bot".into(),
//!     )
//!     .with_channels(vec!["#rust".into()]);
//!     client.initialize().await?;
//!     client.send_message("#rust", "Hello, IRC!").await?;
//!     Ok(())
//! }
//! ```

pub mod channel_type;
pub mod markdown;
pub mod message;
pub mod messenger;
pub mod messengers;
pub mod rich_text;
pub mod server;

pub use channel_type::{ChannelCapabilities, ChannelDescriptor, ChannelType, InboundMode};
pub use markdown::{chunk_markdown_html, markdown_to_slack, markdown_to_telegram_html};
pub use message::{MediaAttachment, Message, SendOptions};
pub use messenger::{Messenger, MessengerManager};
pub use rich_text::{RichText, RichTextNode};
pub use server::ChatServer;
