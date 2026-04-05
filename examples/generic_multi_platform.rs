//! Multi-platform messaging with [`MessengerManager`] and the generic interface.
//!
//! This example shows how to manage several [`GenericMessenger`] instances
//! through a single [`MessengerManager`].  Each messenger is configured
//! independently; swapping a [`MessengerConfig`] variant changes the backend
//! without any other code changes.
//!
//! Here we use three console-backend messengers (no network required).  In a
//! real application you would replace the configs with Discord, Telegram, Slack,
//! etc. configs loaded from a file.
//!
//! Run with:
//!   cargo run --example generic_multi_platform

use chat_system::{
    config::{ConsoleConfig, MessengerConfig},
    GenericMessenger, Messenger, MessengerManager, PresenceStatus, SearchQuery,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== chat-system: multi-platform demo ===\n");

    // ── Build configs ─────────────────────────────────────────────────────────
    // In production these would be deserialized from a config file; the
    // `protocol` field in the JSON/TOML selects the backend.
    let configs: Vec<(&str, MessengerConfig)> = vec![
        // Replace these with real protocol configs as needed:
        //   MessengerConfig::Discord(DiscordConfig { name: "discord".into(), token: "…".into() })
        //   MessengerConfig::Telegram(TelegramConfig { name: "telegram".into(), token: "…".into() })
        //   MessengerConfig::Slack(SlackConfig { name: "slack".into(), token: "…".into() })
        (
            "console-1",
            MessengerConfig::Console(ConsoleConfig {
                name: "console-1".into(),
            }),
        ),
        (
            "console-2",
            MessengerConfig::Console(ConsoleConfig {
                name: "console-2".into(),
            }),
        ),
        (
            "console-3",
            MessengerConfig::Console(ConsoleConfig {
                name: "console-3".into(),
            }),
        ),
    ];

    // ── Initialize all messengers ─────────────────────────────────────────────
    println!("Initializing messengers…");
    let mut mgr = MessengerManager::new();
    for (label, cfg) in configs {
        println!("  {label} (protocol={})", cfg.protocol_name());
        let mut gm = GenericMessenger::new(cfg);
        gm.initialize().await?;
        mgr.add(Box::new(gm));
    }
    println!("  {} messenger(s) ready\n", mgr.messengers().len());

    // ── Presence & text status ────────────────────────────────────────────────
    // Set status on each messenger individually.  Platforms that don't support
    // a given status silently ignore the call.
    println!("Setting presence / text status…");
    for m in mgr.messengers() {
        m.set_status(PresenceStatus::Online).await?;
        m.set_text_status("Online and ready 🟢").await?;
        println!("  {} → online", m.name());
    }

    // ── Broadcast ─────────────────────────────────────────────────────────────
    println!("\nBroadcasting to all platforms…");
    let results = mgr
        .broadcast("#general", "Hello from all platforms via chat-system!")
        .await;
    for (i, res) in results.iter().enumerate() {
        match res {
            Ok(id) => println!("  messenger[{i}] sent  message_id={id}"),
            Err(e) => println!("  messenger[{i}] error: {e}"),
        }
    }

    // ── Receive from all ──────────────────────────────────────────────────────
    println!("\nReceiving from all platforms…");
    let msgs = mgr.receive_all().await?;
    if msgs.is_empty() {
        println!("  (no messages queued)");
    }
    for msg in &msgs {
        println!(
            "  [{}] from={} channel={} content={}",
            msg.timestamp,
            msg.sender,
            msg.channel.as_deref().unwrap_or("?"),
            msg.content,
        );
        if let Some(reactions) = &msg.reactions {
            for r in reactions {
                println!("    reaction: {} × {}", r.emoji, r.count);
            }
        }
        if let Some(parent_id) = &msg.reply_to {
            println!("    (reply to {})", parent_id);
        }
    }

    // ── Search ────────────────────────────────────────────────────────────────
    // Returns empty on platforms without server-side search.
    println!("\nSearching all platforms for \"hello\"…");
    let query = SearchQuery {
        text: "hello".into(),
        channel: Some("#general".into()),
        limit: Some(25),
        ..Default::default()
    };
    for m in mgr.messengers() {
        let hits = m.search_messages(query.clone()).await?;
        println!("  {} → {} hit(s)", m.name(), hits.len());
    }

    // ── Profile pictures ──────────────────────────────────────────────────────
    println!("\nProfile pictures…");
    for m in mgr.messengers() {
        let pic = m.get_profile_picture("alice").await?;
        println!("  {} → get_profile_picture: {pic:?}", m.name());
    }

    // ── Named lookup ──────────────────────────────────────────────────────────
    println!("\nLooking up messenger by name…");
    if let Some(m) = mgr.get("console-2") {
        println!("  found: {} (type={})", m.name(), m.messenger_type());
    }

    // ── Tear down ─────────────────────────────────────────────────────────────
    println!("\nDisconnecting all…");
    mgr.disconnect_all().await?;
    println!("Done.");
    Ok(())
}
