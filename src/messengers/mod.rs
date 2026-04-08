//! Protocol-specific messenger implementations.

mod console;
mod discord;
mod google_chat;
mod imessage;
mod irc;
mod slack;
mod teams;
mod telegram;
mod webhook;

pub use console::ConsoleMessenger;
pub use discord::DiscordMessenger;
pub use google_chat::GoogleChatMessenger;
pub use imessage::IMessageMessenger;
pub use irc::IrcMessenger;
pub use slack::SlackMessenger;
pub use teams::TeamsMessenger;
pub use telegram::TelegramMessenger;
pub use webhook::WebhookMessenger;

#[cfg(feature = "matrix")]
mod matrix;
#[cfg(feature = "matrix")]
pub use matrix::MatrixMessenger;

#[cfg(feature = "matrix-cli")]
mod matrix_cli;
#[cfg(feature = "matrix-cli")]
pub use matrix_cli::{MatrixCliMessenger, MatrixDmConfig};

#[cfg(feature = "signal-cli")]
mod signal_cli;
#[cfg(feature = "signal-cli")]
pub use signal_cli::SignalCliMessenger;

#[cfg(feature = "whatsapp")]
mod whatsapp;
#[cfg(feature = "whatsapp")]
pub use whatsapp::WhatsAppMessenger;

#[cfg(feature = "telegram-cli")]
/// `TelegramCliMessenger` is an alias for [`TelegramMessenger`].
///
/// The `telegram-cli` feature is kept for backward compatibility. New code
/// should use [`TelegramMessenger`] directly.
pub type TelegramCliMessenger = TelegramMessenger;

#[cfg(feature = "discord-cli")]
/// `DiscordCliMessenger` is an alias for [`DiscordMessenger`].
///
/// The `discord-cli` feature is kept for backward compatibility. New code
/// should use [`DiscordMessenger`] directly.  Note that [`DiscordMessenger`]
/// uses the WebSocket Gateway rather than REST polling; `watch_channel` calls
/// are accepted but have no effect since all guild channels are received
/// automatically.
pub type DiscordCliMessenger = DiscordMessenger;

#[cfg(feature = "slack-cli")]
/// `SlackCliMessenger` is an alias for [`SlackMessenger`].
///
/// The `slack-cli` feature is kept for backward compatibility. New code
/// should use [`SlackMessenger`] directly.  The `watch_channel` builder
/// method on [`SlackMessenger`] provides the same channel-filtering
/// behaviour that `SlackCliMessenger` offered.
pub type SlackCliMessenger = SlackMessenger;
