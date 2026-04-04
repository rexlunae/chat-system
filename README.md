# chat-system

A multi-protocol async chat crate for Rust. Provides a **single unified `Messenger` trait** for IRC, Matrix, Discord, Telegram, Slack, Signal, WhatsApp, Microsoft Teams, Google Chat, iMessage, Webhook, and Console — with full rich-text support for every platform's native format.

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

### IRC

#### Standard Connection (Unencrypted)

```rust
use chat_system::messengers::IrcMessenger;
use chat_system::Messenger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut bot = IrcMessenger::new("bot".into(), "irc.libera.chat".into(), 6667, "mybot".into())
        .with_tls(false)
        .with_channels(vec!["#rust".into()]);
    bot.initialize().await?;
    bot.send_message("#rust", "Hello from chat-system!").await?;
    let msgs = bot.receive_messages().await?;
    for m in msgs { println!("{}: {}", m.sender, m.content); }
    bot.disconnect().await?;
    Ok(())
}
```

#### Encrypted Connection (TLS/SSL)

```rust
use chat_system::messengers::IrcMessenger;
use chat_system::Messenger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Encrypted IRC with TLS on port 6697 (standard IRC+TLS port)
    let mut bot = IrcMessenger::new("bot".into(), "irc.libera.chat".into(), 6697, "mybot".into())
        .with_tls(true)  // Enable TLS encryption
        .with_channels(vec!["#rust".into()]);

    bot.initialize().await?;
    bot.send_message("#rust", "Secure message via IRC+TLS!").await?;

    let msgs = bot.receive_messages().await?;
    for m in msgs { println!("{}: {}", m.sender, m.content); }

    bot.disconnect().await?;
    Ok(())
}
```

### Multi-platform via `MessengerManager`

```rust
use chat_system::{Messenger, MessengerManager};
use chat_system::messengers::{IrcMessenger, TelegramMessenger};

let mut mgr = MessengerManager::new();
mgr.add(Box::new(IrcMessenger::new("irc".into(), "irc.libera.chat".into(), 6667, "bot".into())));
mgr.add(Box::new(TelegramMessenger::new("tg".into(), std::env::var("TELEGRAM_TOKEN")?)));
mgr.initialize_all().await?;

loop {
    for msg in mgr.receive_all().await? {
        println!("({}) {}: {}", msg.channel.unwrap_or_default(), msg.sender, msg.content);
    }
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

```rust
use chat_system::ChannelType;

let desc = ChannelType::Slack.descriptor();
println!("{} supports threads: {}", desc.display_name, desc.capabilities.supports_threads); // true

for ct in ChannelType::ALL {
    println!("{:12} inbound={:?}", ct.display_name(), ct.descriptor().capabilities.inbound_mode);
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

## IRC Encryption (TLS/SSL)

### Overview

The IRC messenger supports both unencrypted and encrypted (TLS/SSL) connections for secure communication:

- **Unencrypted**: Standard IRC protocol on port `6667`
- **Encrypted (TLS)**: IRC over TLS on port `6697` (RFC 7194 standard)

### Configuration

```rust
use chat_system::messengers::IrcMessenger;

// Unencrypted
let mut messenger = IrcMessenger::new("name".into(), "irc.server.com".into(), 6667, "nick".into())
    .with_tls(false);

// Encrypted with TLS
let mut messenger = IrcMessenger::new("name".into(), "irc.server.com".into(), 6697, "nick".into())
    .with_tls(true);
```

### Security Features

- **TLS 1.2+**: Modern encryption standards
- **Certificate Verification**: Validates server certificates to prevent MITM attacks
- **Rustls**: Uses pure-Rust TLS implementation for safety and portability

### Server Support

Most public IRC networks support encrypted connections:

| Network | Host | Port | TLS |
| --- | --- | --- | --- |
| Libera.Chat | `irc.libera.chat` | 6667, 6697 | ✓ 6697 |
| Freenode | `irc.freenode.net` | 6667, 6697 | ✓ 6697 |
| EFnet | `irc.mcs.anl.gov` | 6667 | ✗ |
| Undernet | `irc.undernet.org` | 6667, 6697 | ✓ 6697 |

---

## Examples

```sh
cargo run --example irc_client
cargo run --example irc_echo_server
cargo run --example irc_encrypted_client  # TLS/SSL encrypted IRC
cargo run --example discord_bot
cargo run --example matrix_client --features matrix
```

---

## License

MIT. Code adapted from [RustyClaw](https://github.com/rexlunae/RustyClaw) and [Moltis](https://github.com/moltis-org/moltis), both MIT.
