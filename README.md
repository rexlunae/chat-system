# chat-system

A multi-protocol async chat crate for Rust. Provides a **single unified `Messenger` trait** for IRC, Matrix, Discord, Telegram, Slack, Signal, WhatsApp, Microsoft Teams, Google Chat, iMessage, Webhook, and Console — with full rich-text support for every platform's native format.

The primary way to use this crate is through the **generic interface**: `MessengerConfig` is a serde-tagged enum whose `protocol` field selects the backend at runtime, so the protocol is just a field in your config file rather than a compile-time choice.

---

## Features

| Feature flag | Protocols added | Extra dependencies |
|---|---|---|
| *(default)* | IRC, Discord, Telegram, Slack, Teams, Google Chat, iMessage, Webhook, Console | none |
| `matrix` | Matrix (via `matrix-sdk`) | `matrix-sdk` |
| `whatsapp` | WhatsApp (via `wa-rs`) | `wa-rs` family |
| `signal-cli` | Signal (via `signal-cli` subprocess) | *(external binary only)* |
| `full` | All of the above | all optional deps |

---

## Quick Start

```toml
[dependencies]
chat-system = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Generic interface (recommended)

`MessengerConfig` deserializes from any serde-compatible source.  The `protocol` field picks the backend; everything else is the same [`Messenger`] trait regardless of platform.

```rust
use chat_system::{GenericMessenger, Messenger, MessengerConfig, PresenceStatus};
use chat_system::config::IrcConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Build the config programmatically or load it from a file (see below).
    let config = MessengerConfig::Irc(IrcConfig {
        name: "my-bot".into(),
        server: "irc.libera.chat".into(),
        port: 6697,
        nick: "my-bot".into(),
        channels: vec!["#rust".into()],
        tls: true,
    });

    // GenericMessenger is a drop-in Messenger — swap the config to change protocol.
    let mut client = GenericMessenger::new(config);
    client.initialize().await?;

    // Presence + text status (no-op on platforms that don't support them)
    client.set_status(PresenceStatus::Online).await?;
    client.set_text_status("Building something in Rust 🦀").await?;

    client.send_message("#rust", "Hello from chat-system!").await?;

    // Receive messages; reactions are available where the platform supports them
    for msg in client.receive_messages().await? {
        println!("[{}] {}: {}", msg.channel.as_deref().unwrap_or("?"), msg.sender, msg.content);
        if let Some(reactions) = &msg.reactions {
            for r in reactions { println!("  {} × {}", r.emoji, r.count); }
        }
    }

    client.disconnect().await?;
    Ok(())
}
```

### Loading the config from a file

Because `MessengerConfig` derives `serde::Deserialize`, any serde-compatible source works.

**TOML** (`config.toml`):

```toml
protocol = "discord"
name     = "my-bot"
token    = "Bot TOKEN_HERE"
```

```rust
# use chat_system::{GenericMessenger, Messenger, MessengerConfig};
let toml_str = std::fs::read_to_string("config.toml")?;
let config: MessengerConfig = toml::from_str(&toml_str)?;
let mut client = GenericMessenger::new(config);
client.initialize().await?;
```

**JSON** (`config.json`):

```json
{"protocol":"telegram","name":"my-bot","token":"BOT_TOKEN"}
```

```rust
# use chat_system::{GenericMessenger, Messenger, MessengerConfig};
let json_str = std::fs::read_to_string("config.json")?;
let config: MessengerConfig = serde_json::from_str(&json_str)?;
let mut client = GenericMessenger::new(config);
client.initialize().await?;
```

---

## Multi-platform with `MessengerManager`

`MessengerManager` holds multiple `GenericMessenger` instances and broadcasts / receives across all of them at once.

```rust
use chat_system::{GenericMessenger, Messenger, MessengerConfig, MessengerManager};
use chat_system::config::{DiscordConfig, TelegramConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut mgr = MessengerManager::new();
    mgr.add(Box::new(GenericMessenger::new(MessengerConfig::Discord(DiscordConfig {
        name: "discord".into(),
        token: std::env::var("DISCORD_TOKEN")?,
    }))));
    mgr.add(Box::new(GenericMessenger::new(MessengerConfig::Telegram(TelegramConfig {
        name: "telegram".into(),
        token: std::env::var("TELEGRAM_TOKEN")?,
    }))));
    mgr.initialize_all().await?;

    // Broadcast to every connected platform in one call
    mgr.broadcast("#general", "Hello from all platforms!").await;

    // Receive from every platform in one call
    for msg in mgr.receive_all().await? {
        println!("[{}] {}: {}", msg.channel.as_deref().unwrap_or("?"), msg.sender, msg.content);
    }

    mgr.disconnect_all().await?;
    Ok(())
}
```

---

## Reactions

```rust
// Add / remove a reaction (no-op on platforms that don't support it)
client.add_reaction("msg-id-123", "#general", "👍").await?;
client.remove_reaction("msg-id-123", "#general", "👍").await?;

// Reactions arrive on received messages where the platform populates them
for msg in client.receive_messages().await? {
    if let Some(reactions) = &msg.reactions {
        for r in reactions {
            println!("{}: {} ({})", msg.id, r.emoji, r.count);
        }
    }
}
```

---

## Profile pictures

```rust
// Retrieve a user's profile picture URL (None if not supported)
if let Some(url) = client.get_profile_picture("user-id-123").await? {
    println!("Avatar: {url}");
}

// Update the bot's own profile picture
client.set_profile_picture("https://example.com/avatar.png").await?;
```

---

## Replies

```rust
use chat_system::SendOptions;

client.send_message_with_options(SendOptions {
    recipient: "#general",
    content: "Thanks for the message!",
    reply_to: Some("original-message-id"),
    ..Default::default()
}).await?;
```

Incoming reply messages expose the parent ID via `msg.reply_to`.

---

## Search

```rust
use chat_system::SearchQuery;

let results = client.search_messages(SearchQuery {
    text: "deploy".into(),
    channel: Some("#ops".into()),
    limit: Some(20),
    ..Default::default()
}).await?;
for msg in results {
    println!("{}: {}", msg.sender, msg.content);
}
```

---

## Rich Text

```rust
use chat_system::{RichText, RichTextNode};

let msg = RichText(vec![
    RichTextNode::Bold(vec![RichTextNode::Plain("Hello".into())]),
    RichTextNode::Plain(", world! ".into()),
    RichTextNode::Link {
        url: "https://example.com".into(),
        text: vec![RichTextNode::Plain("click".into())],
    },
]);

println!("{}", msg.to_discord_markdown());   // **Hello**, world! [click](https://example.com)
println!("{}", msg.to_telegram_html());      // <b>Hello</b>, world! <a href="…">click</a>
println!("{}", msg.to_slack_mrkdwn());       // *Hello*, world! <https://example.com|click>
println!("{}", msg.to_irc_formatted());      // \x02Hello\x0F, world! click [https://example.com]
println!("{}", msg.to_whatsapp_formatted()); // *Hello*, world! click (https://example.com)
println!("{}", msg.to_matrix_html());        // <b>Hello</b>, world! <a href="…">click</a>

// Parse from Markdown
let rt = RichText::from_markdown("**bold** _italic_ `code`");
```

---

## Channel Capabilities

Each `ChannelType` exposes its feature set so you can decide at runtime what to offer:

```rust
use chat_system::ChannelType;

let caps = ChannelType::Slack.descriptor().capabilities;
println!("Slack supports reactions: {}", caps.supports_reactions);  // true
println!("Slack supports threads: {}",   caps.supports_threads);    // true

for ct in ChannelType::ALL {
    println!("{:14} reactions={} threads={} inbound={:?}",
        ct.display_name(),
        ct.descriptor().capabilities.supports_reactions,
        ct.descriptor().capabilities.supports_threads,
        ct.descriptor().capabilities.inbound_mode);
}
```

---

## Markdown Converters

```rust
use chat_system::markdown::{markdown_to_telegram_html, markdown_to_slack, chunk_markdown_html};

let html  = markdown_to_telegram_html("**bold** and `code`");
// → "<b>bold</b> and <code>code</code>"

let slack = markdown_to_slack("**bold** [link](https://x.com)");
// → "*bold* <https://x.com|link>"

// Split long messages respecting Telegram's 4096-char limit:
let chunks = chunk_markdown_html(&very_long_markdown, 4096);
```

---

## Server

A **server** is a named container of listeners.  It owns no address, port, or protocol — those belong to the listeners.  Different listeners can speak different protocols while feeding into the same handler.

### Programmatic (recommended)

```rust
use chat_system::server::Server;
use chat_system::servers::IrcListener;
use chat_system::ChatServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = Server::new("my-server")
        .add_listener(IrcListener::new("0.0.0.0:6667"))
        .add_listener(IrcListener::new("0.0.0.0:6697"));

    server.run(|msg| async move {
        println!("{}: {}", msg.sender, msg.content);
        Ok(Some(format!("echo: {}", msg.content)))
    }).await?;

    Ok(())
}
```

### Config-driven with `GenericServer`

`ServerConfig` uses the `ListenerConfig` trait (via `typetag`), so listener configs are extensible and can be deserialized from any serde format.

**JSON** (`server.json`):

```json
{
  "name": "my-server",
  "listeners": [
    { "protocol": "irc", "address": "0.0.0.0:6667" },
    { "protocol": "irc", "address": "0.0.0.0:6697" }
  ]
}
```

```rust
use chat_system::{GenericServer, ChatServer, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let json = std::fs::read_to_string("server.json")?;
    let config: ServerConfig = serde_json::from_str(&json)?;
    let mut server = GenericServer::new(config);

    server.run(|msg| async move {
        println!("{}: {}", msg.sender, msg.content);
        Ok(Some(format!("echo: {}", msg.content)))
    }).await?;

    Ok(())
}
```

---

## Examples

```sh
# Generic interface (recommended starting point — no credentials needed)
cargo run --example generic_config_client     # Full API showcase (console backend)
cargo run --example generic_multi_platform    # MessengerManager multi-bot demo

# IRC client + server
cargo run --example irc_echo_server           # Server using Server + IrcListener API
cargo run --example irc_client                # Plaintext client (connects to libera.chat)
cargo run --example irc_encrypted_echo_server # TLS server (raw TLS, needs cert.pem/key.pem)
cargo run --example irc_encrypted_client      # TLS client (connects to libera.chat:6697)

# Other protocols
cargo run --example discord_bot               # Discord bot (needs DISCORD_BOT_TOKEN)
cargo run --example matrix_client --features matrix  # Matrix client (needs credentials)
```

---

## Protocol-specific clients

When you need direct access to protocol-specific features, you can construct the concrete type directly.  The `Messenger` trait is still the primary interface:

```rust
use chat_system::messengers::IrcMessenger;
use chat_system::Messenger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut bot = IrcMessenger::new("bot", "irc.libera.chat", 6697, "mybot")
        .with_tls(true)
        .with_channels(vec!["#rust"]);
    bot.initialize().await?;
    bot.send_message("#rust", "Hello from chat-system!").await?;
    bot.disconnect().await?;
    Ok(())
}
```

### IRC TLS

The IRC messenger supports both plaintext and encrypted connections:

| Mode | Port | Setting |
|---|---|---|
| Plaintext | 6667 | `.with_tls(false)` |
| Encrypted (RFC 7194) | 6697 | `.with_tls(true)` |

```rust
// Plaintext
let mut bot = IrcMessenger::new("name", "irc.server.com", 6667, "nick").with_tls(false);

// TLS
let mut bot = IrcMessenger::new("name", "irc.server.com", 6697, "nick").with_tls(true);
```

| Network | Host | Plaintext port | TLS port |
|---|---|---|---|
| Libera.Chat | `irc.libera.chat` | 6667 | 6697 |
| Freenode | `irc.freenode.net` | 6667 | 6697 |
| Undernet | `irc.undernet.org` | 6667 | 6697 |

---

## License

MIT. Code adapted from [RustyClaw](https://github.com/rexlunae/RustyClaw) and [Moltis](https://github.com/moltis-org/moltis), both MIT.
